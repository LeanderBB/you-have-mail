package dev.lbeernaert.youhavemail.service

import android.app.Activity
import android.app.Notification
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.SharedPreferences
import android.os.Binder
import android.os.IBinder
import android.os.PowerManager
import android.text.Html
import android.text.Spanned
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.localbroadcastmanager.content.LocalBroadcastManager
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import dev.lbeernaert.youhavemail.Account
import dev.lbeernaert.youhavemail.Backend
import dev.lbeernaert.youhavemail.MainActivity
import dev.lbeernaert.youhavemail.NotificationActionClicked
import dev.lbeernaert.youhavemail.NotificationActionDismissed
import dev.lbeernaert.youhavemail.NotificationChannelNewMail
import dev.lbeernaert.youhavemail.NotificationIntentAppNameKey
import dev.lbeernaert.youhavemail.NotificationIntentBackendKey
import dev.lbeernaert.youhavemail.NotificationIntentEmailKey
import dev.lbeernaert.youhavemail.Notifier
import dev.lbeernaert.youhavemail.Proxy
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.ServiceAccountNotificationsStartID
import dev.lbeernaert.youhavemail.ServiceErrorNotificationID
import dev.lbeernaert.youhavemail.ServiceException
import dev.lbeernaert.youhavemail.ServiceNotificationID
import dev.lbeernaert.youhavemail.createAccountErrorNotification
import dev.lbeernaert.youhavemail.createAccountStatusNotification
import dev.lbeernaert.youhavemail.createNotificationChannels
import dev.lbeernaert.youhavemail.createServiceErrorNotification
import dev.lbeernaert.youhavemail.createServiceNotification
import dev.lbeernaert.youhavemail.initLog
import dev.lbeernaert.youhavemail.migrateOldConfig
import dev.lbeernaert.youhavemail.newEncryptionKey
import dev.lbeernaert.youhavemail.newService
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.cancel
import java.io.File
import java.time.Duration
import java.util.concurrent.TimeUnit
import java.util.concurrent.locks.Lock
import java.util.concurrent.locks.ReentrantLock


const val serviceLogTag = "observer"

data class NotificationIds(val newMessages: Int, val statusUpdate: Int, val errors: Int)

data class UnreadState(var unreadCount: UInt, var lines: ArrayList<Spanned>)

fun getLogPath(context: Context): File {
    return File(context.filesDir.canonicalPath, "logs")
}

class ObserverService : Service(), Notifier {
    private var wakeLock: PowerManager.WakeLock? = null
    private var isServiceStarted = false
    private val binder = LocalBinder()
    private val coroutineScope = CoroutineScope(
        Dispatchers.Default
    )
    private val mReceiver = Receiver(this)

    // Notification State
    private var notificationIDCounter: Int = ServiceAccountNotificationsStartID
    private var notificationMap: HashMap<String, NotificationIds> = HashMap()
    private var unreadState: HashMap<String, UnreadState> = HashMap()
    private var unreadMessageStateMutex: Lock = ReentrantLock()

    // Have to keep this here or it won't survive activity refreshes
    private var mInLoginAccount: Account? = null
    private var mInLoginProxy: Proxy? = null
    private var mBackends: ArrayList<Backend> = ArrayList()

    var mService: dev.lbeernaert.youhavemail.Service? = null

    var mServiceState: ObserverServiceState = ObserverServiceState()

    inner class LocalBinder : Binder() {
        fun getService(): ObserverService = this@ObserverService
    }

    override fun onBind(intent: Intent?): IBinder {
        Log.d(serviceLogTag, "Some component wants to bind with the service")
        return binder
    }

    override fun onUnbind(intent: Intent?): Boolean {
        Log.d(serviceLogTag, "Some component unbound from the system")
        return super.onUnbind(intent)
    }


    fun setInLoginAccount(account: Account) {
        mInLoginAccount?.destroy()
        mInLoginAccount = account
    }

    fun setInLoginProxy(proxy: Proxy?) {
        mInLoginProxy = proxy
    }

    fun getInLoginProxy(): Proxy? {
        return mInLoginProxy
    }

    fun getInLoginAccount(): Account? {
        return mInLoginAccount
    }

    fun getBackends(): List<Backend> {
        return mBackends
    }

    fun setPollInterval(interval: ULong) {
        mService!!.setPollInterval(interval)
        mServiceState.onPollIntervalChanged(interval)
        registerWorker(this, Duration.ofSeconds(interval.toLong()).toMinutes())
    }

    fun setAccountProxy(email: String, proxy: Proxy?) {
        mService!!.setAccountProxy(email, proxy)
    }

    fun pauseServiceNoNetwork() {
        Log.d(serviceLogTag, "Pause service")
        mServiceState.onNetworkLost()
        try {
            mService!!.pause()
            updateServiceNotificationStatus("Paused (no network)")
        } catch (e: ServiceException) {
            Log.e(serviceLogTag, "Failed to pause service: $e")
        }
    }

    fun resumeService() {
        mServiceState.onNetworkRestored()
        Log.d(serviceLogTag, "Network restored")
        try {
            mService!!.resume()
            updateServiceNotificationStatus("Paused (no network)")
            updateServiceNotificationStatus("Running")
        } catch (e: ServiceException) {
            Log.e(serviceLogTag, "Failed to resume service: $e")
            createAndDisplayServiceErrorNotification(
                "Failed to resume Service, please restart manually",
                e
            )
        }
    }

    fun acquireWakeLock() {
        wakeLock?.let {
            if (!it.isHeld) {
                it.acquire(TimeUnit.MINUTES.toMillis(5))
            }
        }
    }

    fun releaseWakeLock() {
        wakeLock?.let {
            if (it.isHeld) {
                it.release()
            }
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Log.d(serviceLogTag, "onStartCommand executed with startId: $startId")
        if (intent != null) {
            val action = intent.action
            Log.d(serviceLogTag, "Using intent with action $action")
            when (action) {
                Actions.START.name -> startService()
                Actions.STOP.name -> stopService()
                else -> Log.e(serviceLogTag, "Unknown action:  $action")
            }
        } else {
            Log.d(serviceLogTag, "Null intent, probably restarted by the system")
        }

        return START_STICKY
    }

    private fun getConfigFilePathV2(): String {
        return filesDir.canonicalPath + "/config_v2"
    }

    override fun onCreate() {
        super.onCreate()

        // we need this lock so our service does not affected by Doze mode
        wakeLock = (getSystemService(Context.POWER_SERVICE) as PowerManager).run {
            newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "YouHaveMailService::lock")
        }

        LocalBroadcastManager.getInstance(this)
            .registerReceiver(mReceiver, IntentFilter(POLL_INTENT_NAME))

        val notificationManager =
            getSystemService(NOTIFICATION_SERVICE) as NotificationManager

        createNotificationChannels(notificationManager)

        try {
            val logErr = initLog(getLogPath(this).canonicalPath)
            if (logErr != null) {
                createAndDisplayServiceErrorNotification(logErr)
            }
        } catch (e: Exception) {
            createAndDisplayServiceErrorNotification("Failed to init log: ${e}")
        }

        val configPath = getConfigFilePathV2()
        val encryptionKey = loadEncryptionKey(this)

        try {
            val config = loadConfig(this)
            if (config != null) {
                Log.d(serviceLogTag, "Found old config migrating")
                migrateOldConfig(encryptionKey, config, configPath)
                deleteOldConfig(this)
            }
        } catch (e: ServiceException) {
            Log.e(serviceLogTag, "Failed to migrate old file $e")
            createAndDisplayServiceErrorNotification(
                "Failed to migrate old config, state will be reset",
                e
            )
        }

        Log.d(serviceLogTag, "Service has been created")
        startForeground(ServiceNotificationID, createServiceNotification(this, "Running"))
        try {
            mService = newService(this, encryptionKey, configPath)
            mBackends.addAll(mService!!.getBackends())
        } catch (e: ServiceException) {
            Log.e(serviceLogTag, "Failed to create service:$e")
            createAndDisplayServiceErrorNotification(
                "Failed to create Service",
                e
            )
            return
        }

        mServiceState.setFrom(mService!!.getObservedAccounts())
        val pollInterval = mService!!.getPollInterval()
        val pollIntervalMin = Duration.ofSeconds(pollInterval.toLong()).toMinutes()
        mServiceState.onPollIntervalChanged(pollInterval)
        registerWorker(this, pollIntervalMin)
    }

    override fun onDestroy() {
        LocalBroadcastManager.getInstance(this)
            .unregisterReceiver(mReceiver)
        coroutineScope.cancel()
        mInLoginAccount?.destroy()
        mBackends.forEach {
            it.destroy()
        }
        mBackends.clear()
        mService?.destroy()
        Log.d(serviceLogTag, "The service has been destroyed")
        super.onDestroy()
    }

    private fun startService() {
        if (isServiceStarted) return
        Log.d(serviceLogTag, "Starting foreground service task")

        isServiceStarted = true
        setServiceState(this, ServiceState.STARTED)

    }

    private fun stopService() {
        Log.d(serviceLogTag, "Stopping foreground service")
        try {
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
        } catch (e: java.lang.Exception) {
            Log.e(serviceLogTag, "Service stopped without being started: ${e.message}")
        }
        isServiceStarted = false
        setServiceState(this, ServiceState.STOPPED)
    }

    private fun createAlertNotification(
        email: String,
        backend: String,
        unreadState: UnreadState,
    ): Notification {

        val appName = getAppNameForBackend(backend)

        val clickIntent: PendingIntent? =
            if (appName != null) {
                Intent(this, MainActivity::class.java).let { intent ->
                    intent.action = NotificationActionClicked
                    intent.putExtra(
                        NotificationIntentEmailKey, email
                    )
                    intent.putExtra(
                        NotificationIntentBackendKey, backend
                    )
                    intent.putExtra(
                        NotificationIntentAppNameKey,
                        appName
                    )
                    intent.addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP)
                    intent.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP)
                    PendingIntent.getActivity(
                        this,
                        0,
                        intent,
                        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
                    )
                }
            } else {
                Log.d(serviceLogTag, "No app found for backed '$backend'. No notification action")
                null
            }

        val dismissIntent: PendingIntent =
            Intent().let { intent ->
                intent.action = NotificationActionDismissed
                intent.putExtra(
                    NotificationIntentEmailKey, email
                )
                intent.putExtra(
                    NotificationIntentBackendKey, backend
                )
                PendingIntent.getBroadcast(this, 0, intent, PendingIntent.FLAG_IMMUTABLE)
            }


        val builder: NotificationCompat.Builder = NotificationCompat.Builder(
            this,
            NotificationChannelNewMail
        )

        if (clickIntent != null) {
            builder.setContentIntent(clickIntent)
        }

        if (unreadState.lines.size == 1) {
            return builder
                .setContentTitle("$email has 1 new message")
                .setContentText(unreadState.lines[0])
                .setDeleteIntent(dismissIntent)
                .setAutoCancel(true)
                .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
                .setSmallIcon(R.drawable.ic_stat_alert)
                .setTicker("You Have Mail Alert")
                .build()
        }

        var style = NotificationCompat.InboxStyle()

        for (line in unreadState.lines.reversed()) {
            style = style.addLine(line)
        }

        return builder
            .setContentTitle("$email has ${unreadState.unreadCount} new message(s)")
            .setDeleteIntent(dismissIntent)
            .setStyle(style)
            .setAutoCancel(true)
            .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
            .setSmallIcon(R.drawable.ic_stat_alert)
            .setTicker("You Have Mail Alert")
            .build()
    }

    private fun genNotificationID(): Int {
        return System.currentTimeMillis().toInt()
    }

    private fun getOrCreateNotificationIDs(email: String): NotificationIds {
        val ids = notificationMap[email]
        if (ids != null) {
            return ids
        }

        val newIds = NotificationIds(
            newMessages = notificationIDCounter++,
            statusUpdate = notificationIDCounter++,
            errors = notificationIDCounter++
        )

        notificationMap[email] = newIds
        return newIds
    }

    private fun getAndUpdateUnreadMessageCount(
        email: String,
        newMessageCount: UInt,
        line: Spanned,
        reset: Boolean
    ): UnreadState {
        val result: UnreadState
        unreadMessageStateMutex.lock()
        if (reset) {
            val state = UnreadState(newMessageCount, arrayListOf(line))
            unreadState[email] = state
            result = state
        } else {
            var state = unreadState.getOrDefault(email, UnreadState(0u, arrayListOf()))
            state.unreadCount += newMessageCount
            state.lines.add(line)
            unreadState[email] = state
            result = state
        }
        unreadMessageStateMutex.unlock()
        return result
    }

    private fun isNotificationVisible(id: Int): Boolean {
        with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            for (n in this.activeNotifications) {
                if (n.id == id) {
                    return true
                }
            }
        }

        return false
    }

    override fun newEmail(account: String, backend: String, sender: String, subject: String) {
        Log.d(serviceLogTag, "New Mail: $account ($backend)")
        try {
            val ids = getOrCreateNotificationIDs(account)
            val isNotificationActive = isNotificationVisible(ids.newMessages)
            val styleText: Spanned =
                Html.fromHtml("<b>$sender:</b> $subject", Html.FROM_HTML_MODE_LEGACY)
            val unreadState =
                getAndUpdateUnreadMessageCount(account, 1u, styleText, !isNotificationActive)
            val notification =
                createAlertNotification(account, backend, unreadState)
            with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
                if (this.areNotificationsEnabled()) {
                    notify(ids.newMessages, notification)
                }
            }
        } catch (e: Exception) {
            Log.e(serviceLogTag, "Failed to display notification: $e")
        }
    }

    override fun accountAdded(email: String, backend: String, proxy: Proxy?) {
        mServiceState.onAccountAdded(email, backend, proxy)
        Log.d(serviceLogTag, "Account added: $email")
    }

    override fun accountLoggedOut(email: String) {
        mServiceState.onAccountLoggedOut(email)
        Log.d(serviceLogTag, "Account Logged Out: $email")
        try {
            val notification =
                createAccountStatusNotification(this, "Account $email session expired")
            val ids = getOrCreateNotificationIDs(email)
            with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
                if (this.areNotificationsEnabled()) {
                    notify(ids.statusUpdate, notification)
                }
            }
        } catch (e: Exception) {
            Log.e(serviceLogTag, "Failed to display notification: $e")
        }
    }

    override fun accountRemoved(email: String) {
        mServiceState.onAccountRemoved(email)
        Log.d(serviceLogTag, "Account Removed: $email")
    }

    override fun accountOffline(email: String) {
        mServiceState.onAccountOffline(email)
        Log.d(serviceLogTag, "Account Offline: $email")
    }

    override fun accountOnline(email: String) {
        mServiceState.onAccountOnline(email)
        Log.d(serviceLogTag, "Account Online: $email")
    }

    override fun accountError(email: String, error: ServiceException) {
        Log.e(serviceLogTag, "Account Error: $email => $error")
        try {
            val notification = createAccountErrorNotification(this, email, error)
            val ids = getOrCreateNotificationIDs(email)
            with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
                if (this.areNotificationsEnabled()) {
                    notify(ids.errors, notification)
                }
            }
        } catch (e: Exception) {
            Log.e(serviceLogTag, "Failed to display notification: $e")
        }
    }

    override fun proxyApplied(email: String, proxy: Proxy?) {
        mServiceState.onAccountProxyChanged(email, proxy)
        Log.d(serviceLogTag, "Account $email applied Proxy $proxy")
    }

    override fun error(msg: String) {
        try {
            Log.e(serviceLogTag, "Service Error: $msg")
            createAndDisplayServiceErrorNotification(msg)
        } catch (e: Exception) {
            Log.e(serviceLogTag, "Failed to display notification: $e")
        }
    }

    private fun loadConfig(context: Context): String? {
        Log.d(serviceLogTag, "Loading Old Config")
        val preferences = getSharedPreferences(context)
        return preferences.getString("CONFIG", null)
    }

    private fun deleteOldConfig(context: Context) {
        Log.d(serviceLogTag, "Deleting Old Config")
        val preferences = getSharedPreferences(context)
        preferences.edit().remove("CONFIG").apply()
    }

    private fun loadEncryptionKey(context: Context): String {
        Log.d(serviceLogTag, "Loading Encryption key")
        val preferences = getSharedPreferences(context)
        val key = preferences.getString("KEY", null)
        return if (key == null) {
            Log.d(serviceLogTag, "No key exists, recording")
            val newKey = newEncryptionKey()
            preferences.edit().putString("KEY", newKey).apply()
            newKey
        } else {
            key
        }
    }

    private fun getSharedPreferences(context: Context): SharedPreferences {
        val masterKey = MasterKey.Builder(context, MasterKey.DEFAULT_MASTER_KEY_ALIAS)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()

        return EncryptedSharedPreferences.create(
            context,
            // passing a file name to share a preferences
            "preferences",
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )
    }

    private fun updateServiceNotificationStatus(newState: String) {
        val notification = createServiceNotification(this, newState)
        with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            if (this.areNotificationsEnabled()) {
                notify(ServiceNotificationID, notification)
            }
        }
    }

    fun createAndDisplayServiceErrorNotification(
        text: String,
        exception: ServiceException
    ) {
        val notification = createServiceErrorNotification(this, text, exception)
        with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            if (this.areNotificationsEnabled()) {
                notify(ServiceErrorNotificationID, notification)
            }
        }
    }

    fun createAndDisplayServiceErrorNotification(
        text: String,
    ) {
        val notification = createServiceErrorNotification(this, text)
        with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            if (this.areNotificationsEnabled()) {
                notify(ServiceErrorNotificationID, notification)
            }
        }
    }

}

class Receiver(private val mObserver: ObserverService) : BroadcastReceiver() {
    override fun onReceive(context: Context?, intent: Intent?) {
        if (intent == null) {
            return
        }
        if (intent.action == null) {
            return
        }
        if (intent.action != POLL_INTENT_NAME) {
            return
        }

        Log.d(serviceLogTag, "Received poll broadcast")
        if (mObserver.mService == null) {
            mObserver.createAndDisplayServiceErrorNotification("Received request to poll, but no service available")
        }
        try {
            mObserver.acquireWakeLock()
            mObserver.mService!!.poll()
        } catch (e: ServiceException) {
            mObserver.createAndDisplayServiceErrorNotification("Failed to poll accounts", e)
        } finally {
            mObserver.releaseWakeLock()
        }
    }
}


fun getAppNameForBackend(backend: String): String? {
    return when (backend) {
        "Proton Mail" -> "ch.protonmail.android"
        "Proton Mail V-Other" -> "ch.protonmail.android"
        "Null Backend" -> "ch.protonmail.android"

        else -> null
    }
}