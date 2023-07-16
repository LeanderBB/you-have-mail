package dev.lbeernaert.youhavemail.service

import android.app.Activity
import android.app.Notification
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.os.Binder
import android.os.IBinder
import android.os.PowerManager
import android.text.Html
import android.text.Spanned
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
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
import dev.lbeernaert.youhavemail.ObserverAccount
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
import dev.lbeernaert.youhavemail.migrateOldConfig
import dev.lbeernaert.youhavemail.newEncryptionKey
import dev.lbeernaert.youhavemail.newService
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import java.time.LocalDateTime
import java.util.concurrent.locks.Lock
import java.util.concurrent.locks.ReentrantLock


const val serviceLogTag = "observer"

data class NotificationIds(val newMessages: Int, val statusUpdate: Int, val errors: Int)

data class AccountActivity(val dateTime: LocalDateTime, val status: String)

data class UnreadState(var unreadCount: UInt, var lines: ArrayList<Spanned>)

class ObserverService : Service(), Notifier {
    private var wakeLock: PowerManager.WakeLock? = null
    private var isServiceStarted = false
    private val binder = LocalBinder()
    private val coroutineScope = CoroutineScope(
        Dispatchers.Default
    )

    // System listeners
    private val networkListener = NetworkListener(this)

    // State Flows
    private var _accountListFlow: MutableStateFlow<List<ObserverAccount>> =
        MutableStateFlow(ArrayList())
    val accountList: StateFlow<List<ObserverAccount>> get() = _accountListFlow
    private var _pollIntervalFlow: MutableStateFlow<ULong> =
        MutableStateFlow(15UL)
    val pollInterval: StateFlow<ULong> get() = _pollIntervalFlow

    // Notification State
    private var notificationIDCounter: Int = ServiceAccountNotificationsStartID
    private var notificationMap: HashMap<String, NotificationIds> = HashMap()
    private var unreadState: HashMap<String, UnreadState> = HashMap()
    private var unreadMessageStateMutex: Lock = ReentrantLock()

    // Account Activity
    private var accountActivity = HashMap<String, ArrayList<AccountActivity>>()
    private var accountActivityLock: Lock = ReentrantLock()

    // Have to keep this here or it won't survive activity refreshes
    private var mInLoginAccount: Account? = null
    private var mInLoginProxy: Proxy? = null
    private var mBackends: ArrayList<Backend> = ArrayList()

    var mService: dev.lbeernaert.youhavemail.Service? = null

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
        _pollIntervalFlow.value = interval
    }

    fun setAccountProxy(email: String, proxy: Proxy?) {
        mService!!.setAccountProxy(email, proxy)
    }

    fun pauseServiceNoNetwork() {
        Log.d(serviceLogTag, "Pause service")
        try {
            mService!!.pause()
            updateServiceNotificationStatus("Paused (no network)")
        } catch (e: ServiceException) {
            Log.e(serviceLogTag, "Failed to pause service: $e")
        }
        recordAccountActivityAll("Paused due to lack of network")
    }

    fun resumeService() {
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
        recordAccountActivityAll("Resumed, network restored")
    }

    fun acquireWakeLock() {
        wakeLock?.let {
            if (!it.isHeld) {
                it.acquire()
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

        val notificationManager =
            getSystemService(NOTIFICATION_SERVICE) as NotificationManager

        createNotificationChannels(notificationManager)

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
            _pollIntervalFlow.value = mService!!.getPollInterval()
            updateAccountList()
            registerNetworkListener()
        } catch (e: ServiceException) {
            Log.e(serviceLogTag, "Failed to create service:$e")
            createAndDisplayServiceErrorNotification(
                "Failed to create Service",
                e
            )
        }
    }

    override fun onDestroy() {
        unregisterNetworkListener()
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

        // we need this lock so our service does not affected by Doze mode
        wakeLock = (getSystemService(Context.POWER_SERVICE) as PowerManager).run {
            newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "YouHaveMailService::lock").apply {
                acquire()
            }
        }
    }

    private fun stopService() {
        Log.d(serviceLogTag, "Stopping foreground service")
        try {
            releaseWakeLock()
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
        } catch (e: java.lang.Exception) {
            Log.e(serviceLogTag, "Service stopped without being started: ${e.message}")
        }
        isServiceStarted = false
        setServiceState(this, ServiceState.STOPPED)
    }


    private fun registerNetworkListener() {
        val request =
            NetworkRequest.Builder().addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
                .addTransportType(NetworkCapabilities.TRANSPORT_WIFI)
                .addTransportType(NetworkCapabilities.TRANSPORT_CELLULAR)
                .build()

        val connectivityManager =
            ContextCompat.getSystemService(
                this,
                ConnectivityManager::class.java
            ) as ConnectivityManager
        connectivityManager.requestNetwork(request, networkListener)
    }

    private fun unregisterNetworkListener() {
        val connectivityManager =
            ContextCompat.getSystemService(
                this,
                ConnectivityManager::class.java
            ) as ConnectivityManager
        connectivityManager.unregisterNetworkCallback(networkListener)
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

    override fun accountAdded(email: String) {
        Log.d(serviceLogTag, "Account added: $email")
        addAccountActivity(email)
        updateAccountList()
    }

    override fun accountLoggedOut(email: String) {
        Log.d(serviceLogTag, "Account Logged Out: $email")
        try {
            recordAccountActivity(email, "Session Expired")
            updateAccountList()
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
        Log.d(serviceLogTag, "Account Removed: $email")
        updateAccountList()
        removeAccountActivity(email)
    }

    override fun accountOffline(email: String) {
        Log.d(serviceLogTag, "Account Offline: $email")
        recordAccountActivity(email, "Offline")
        updateAccountList()
    }

    override fun accountOnline(email: String) {
        Log.d(serviceLogTag, "Account Online: $email")
        recordAccountActivity(email, "Online")
        updateAccountList()
    }

    override fun accountError(email: String, error: ServiceException) {
        Log.e(serviceLogTag, "Account Error: $email => $error")
        try {
            recordAccountActivity(email, error.toString())
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
        Log.d(serviceLogTag, "Account $email applied Proxy $proxy")
        updateAccountList()
        recordAccountActivity(email, "Proxy settings changed")
    }

    override fun error(msg: String) {
        try {
            Log.e(serviceLogTag, "Service Error: $msg")
            createAndDisplayServiceErrorNotification(msg)
        } catch (e: Exception) {
            Log.e(serviceLogTag, "Failed to display notification: $e")
        }
    }

    private fun updateAccountList() {
        coroutineScope.launch {
            if (mService != null) {
                try {
                    val accounts = mService!!.getObservedAccounts()
                    _accountListFlow.value = accounts
                } catch (e: ServiceException) {
                    Log.e(serviceLogTag, "Failed to refresh account list")
                }
            }
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

    private fun createAndDisplayServiceErrorNotification(
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

    private fun createAndDisplayServiceErrorNotification(
        text: String,
    ) {
        val notification = createServiceErrorNotification(this, text)
        with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            if (this.areNotificationsEnabled()) {
                notify(ServiceErrorNotificationID, notification)
            }
        }
    }


    fun getAccountActivity(email: String): List<AccountActivity> {
        accountActivityLock.lock()

        var result = if (accountActivity.containsKey(email)) {
            ArrayList(accountActivity[email]!!)
        } else {
            ArrayList()
        }

        accountActivityLock.unlock()
        return result
    }

    private fun recordAccountActivity(email: String, message: String) {
        try {
            accountActivityLock.lock()

            val newActivity = AccountActivity(dateTime = LocalDateTime.now(), message)

            var list = accountActivity.getOrPut(email) { ArrayList() }

            if (list.size > 20) {
                list.removeAt(0)
            }
            list.add(newActivity)

        } catch (e: Exception) {
            Log.e(serviceLogTag, "Failed to record activity: $e")
        } finally {
            accountActivityLock.unlock()
        }
    }

    private fun recordAccountActivityAll(message: String) {
        try {
            accountActivityLock.lock()

            val newActivity = AccountActivity(dateTime = LocalDateTime.now(), message)

            for (list in accountActivity.values) {
                if (list.size > 20) {
                    list.removeAt(0)
                }
                list.add(newActivity)
            }

        } catch (e: Exception) {
            Log.e(serviceLogTag, "Failed to record activity: $e")
        } finally {
            accountActivityLock.unlock()
        }
    }

    private fun removeAccountActivity(email: String) {
        try {
            accountActivityLock.lock()
            accountActivity.remove(email)
        } catch (e: Exception) {
            Log.e(serviceLogTag, "Failed to remove activity: $e")
        } finally {
            accountActivityLock.unlock()
        }
    }

    private fun addAccountActivity(email: String) {
        try {
            accountActivityLock.lock()
            accountActivity[email] = kotlin.collections.ArrayList()
        } catch (e: Exception) {
            Log.e(serviceLogTag, "Failed to remove activity: $e")
        } finally {
            accountActivityLock.unlock()
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