package dev.lbeernaert.youhavemail.service

import android.app.*
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.graphics.Color
import android.media.AudioAttributes
import android.media.RingtoneManager
import android.os.Binder
import android.os.IBinder
import android.os.PowerManager
import android.util.Log
import android.widget.Toast
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import dev.lbeernaert.youhavemail.*
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import java.util.concurrent.locks.Lock
import java.util.concurrent.locks.ReentrantLock


const val serviceLogTag = "observer"

data class NotificationIds(val newMessages: Int, val statusUpdate: Int, val errors: Int)

class ObserverService : Service(), Notifier, ServiceFromConfigCallback {
    private var wakeLock: PowerManager.WakeLock? = null
    private var isServiceStarted = false
    private val binder = LocalBinder()
    private val coroutineScope = CoroutineScope(
        Dispatchers.Default
    )
    private var _accountListFlow: MutableStateFlow<List<ObserverAccount>> =
        MutableStateFlow(ArrayList())
    val accountList: StateFlow<List<ObserverAccount>> get() = _accountListFlow

    private var notificationIDCounter: Int = 2
    private var notificationMap: HashMap<String, NotificationIds> = HashMap()
    private var unreadMessageCounts: HashMap<String, UInt> = HashMap()
    private var unreadMessageCountMutex: Lock = ReentrantLock()

    // Have to keep this here or it won't survive activity refreshes
    private var mInLoginAccount: Account? = null
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

    fun getInLoginAccount(): Account? {
        return mInLoginAccount
    }

    fun getBackends(): List<Backend> {
        return mBackends
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

    override fun onCreate() {
        super.onCreate()

        createNotificationChannel()

        Log.d(serviceLogTag, "Service has been created")
        startForeground(1, createServiceNotification())
        try {
            val config = loadConfig(this)
            mService = if (config == null) {
                Log.d(serviceLogTag, "No config found, starting fresh")
                newService(this)
            } else {
                Log.d(serviceLogTag, "Starting service with config")
                newServiceFromConfig(this, this, config)
            }
            mBackends.addAll(mService!!.getBackends())
            updateAccountList()
        } catch (e: ServiceException) {
            Log.e(serviceLogTag, "Failed to create service:$e")
        }
    }

    override fun onDestroy() {
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
            wakeLock?.let {
                if (it.isHeld) {
                    it.release()
                }
            }
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
        } catch (e: java.lang.Exception) {
            Log.e(serviceLogTag, "Service stopped without being started: ${e.message}")
        }
        isServiceStarted = false
        setServiceState(this, ServiceState.STOPPED)
    }

    private fun createNotificationChannel() {
        val notificationManager =
            getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

        val group = notificationManager.createNotificationChannelGroup(
            NotificationChannelGroup(
                NotificationGroupNewEmails,
                "New Emails"
            )
        )

        val channelService = NotificationChannel(
            NotificationChannelService,
            "Background Service ",
            NotificationManager.IMPORTANCE_LOW
        ).let {
            it.description = "You Have Mail Background Service"
            it.enableLights(false)
            it.enableVibration(false)
            it
        }
        notificationManager.createNotificationChannel(channelService)

        val channelAlerter = NotificationChannel(
            NotificationChannelNewMail,
            "New Emails and Status",
            NotificationManager.IMPORTANCE_HIGH
        ).let {
            it.description = "You Have Mail Notifications"
            it.enableLights(true)
            it.lightColor = Color.WHITE
            it.enableVibration(true)
            it.group = NotificationGroupNewEmails
            it.setSound(
                RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION),
                AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_NOTIFICATION)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                    .build()
            )
            it
        }
        notificationManager.createNotificationChannel(channelAlerter)

        val channelErrors = NotificationChannel(
            NotificationChannelError,
            "Errors",
            NotificationManager.IMPORTANCE_HIGH
        ).let {
            it.description = "You Have Mail Errors"
            it.enableLights(true)
            it.lightColor = Color.RED
            it.enableVibration(true)
            it.setSound(
                RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION),
                AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_NOTIFICATION)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                    .build()
            )
            it
        }
        notificationManager.createNotificationChannel(channelErrors)
    }

    private fun createServiceNotification(): Notification {
        val builder: Notification.Builder = Notification.Builder(
            this,
            NotificationChannelService,
        )

        return builder
            .setContentTitle("You Have Mail")
            .setContentText("Background Service Running")
            .setSmallIcon(R.drawable.ic_stat_sync)
            .setVisibility(Notification.VISIBILITY_SECRET)
            .setCategory(Notification.CATEGORY_SERVICE)
            .setOngoing(true)
            .setTicker("You Have Mail Service")
            .build()
    }

    private fun createAlertNotification(
        email: String,
        backend: String,
        messageCount: UInt
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


        val builder: Notification.Builder = Notification.Builder(
            this,
            NotificationChannelNewMail
        )

        if (clickIntent != null) {
            builder.setContentIntent(clickIntent)
        }

        return builder
            .setContentTitle("You Have Mail")
            .setContentText("$email has $messageCount new message(s)")
            .setDeleteIntent(dismissIntent)
            .setAutoCancel(true)
            .setVisibility(Notification.VISIBILITY_PRIVATE)
            .setSmallIcon(R.drawable.ic_stat_alert)
            .setTicker("You Have Mail Alert")
            .build()
    }

    private fun createAccountErrorNotification(email: String, err: ServiceException): Notification {
        val pendingIntent: PendingIntent =
            Intent(this, MainActivity::class.java).let { notificationIntent ->
                PendingIntent.getActivity(this, 0, notificationIntent, PendingIntent.FLAG_IMMUTABLE)
            }

        val builder: Notification.Builder = Notification.Builder(
            this,
            NotificationChannelError
        )

        val errorString = serviceExceptionToErrorStr(err, email)

        return builder
            .setContentTitle("You Have Mail")
            .setContentText("$email error: $errorString")
            .setContentIntent(pendingIntent)
            .setAutoCancel(true)
            .setVisibility(Notification.VISIBILITY_PRIVATE)
            .setSmallIcon(R.drawable.ic_stat_err)
            .setTicker("You Have Mail Alert")
            .build()
    }

    private fun createAccountStatusNotification(text: String): Notification {
        val pendingIntent: PendingIntent =
            Intent(this, MainActivity::class.java).let { notificationIntent ->
                PendingIntent.getActivity(
                    this,
                    0,
                    notificationIntent,
                    PendingIntent.FLAG_MUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
                )
            }

        val builder: Notification.Builder = Notification.Builder(
            this,
            NotificationChannelNewMail,
        )

        return builder
            .setContentTitle("You Have Mail")
            .setContentText(text)
            .setAutoCancel(true)
            .setContentIntent(pendingIntent)
            .setVisibility(Notification.VISIBILITY_PRIVATE)
            .setSmallIcon(R.drawable.ic_stat_alert)
            .setTicker("You Have Mail Alert")
            .build()
    }


    private fun getOrCreateNotificationIDs(email: String): NotificationIds {
        val ids = notificationMap[email]
        if (ids != null) {
            return ids
        }

        var newIds = NotificationIds(
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
        reset: Boolean
    ): UInt {
        var result = 0u
        unreadMessageCountMutex.lock()
        if (reset) {
            unreadMessageCounts[email] = newMessageCount
            result = newMessageCount
        } else {
            var count = unreadMessageCounts.getOrDefault(email, 0u)
            count += newMessageCount
            unreadMessageCounts[email] = count
            result = count
        }
        unreadMessageCountMutex.unlock()
        return result
    }

    private fun resetUnreadMessageCount(email: String) {
        unreadMessageCountMutex.lock()
        Log.d(serviceLogTag, "Resetting message count $email current=${unreadMessageCounts[email]}")
        unreadMessageCounts[email] = 0u
        unreadMessageCountMutex.unlock()
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


    override fun newEmail(account: String, backend: String, count: UInt) {
        Log.d(serviceLogTag, "New Mail: $account ($backend) num=$count")
        val ids = getOrCreateNotificationIDs(account)
        val isNotificationActive = isNotificationVisible(ids.newMessages)
        val unreadCount = getAndUpdateUnreadMessageCount(account, count, !isNotificationActive)
        val notification = createAlertNotification(account, backend, unreadCount)
        with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            if (this.areNotificationsEnabled()) {
                notify(ids.newMessages, notification)
            }
        }
    }

    override fun accountAdded(email: String) {
        Log.d(serviceLogTag, "Account added: $email")
        updateAccountList()
    }

    override fun accountLoggedOut(email: String) {
        Log.d(serviceLogTag, "Account Logged Out: $email")
        updateAccountList()
        val notification = createAccountStatusNotification("Account $email session expired")
        val ids = getOrCreateNotificationIDs(email)
        with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            if (this.areNotificationsEnabled()) {
                notify(ids.statusUpdate, notification)
            }
        }
    }

    override fun accountRemoved(email: String) {
        Log.d(serviceLogTag, "Account Removed: $email")
        updateAccountList()
    }

    override fun accountOffline(email: String) {
        Log.d(serviceLogTag, "Account Offline: $email")
        updateAccountList()
    }

    override fun accountOnline(email: String) {
        Log.d(serviceLogTag, "Account Online: $email")
        updateAccountList()
    }

    override fun accountError(email: String, error: ServiceException) {
        Log.e(serviceLogTag, "Account Error: $email => $error")
        val notification = createAccountErrorNotification(email, error)
        val ids = getOrCreateNotificationIDs(email)
        with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            if (this.areNotificationsEnabled()) {
                notify(ids.errors, notification)
            }
        }
    }

    private fun updateAccountList() {
        val context = this
        coroutineScope.launch {
            if (mService != null) {
                storeConfig(context)
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
        Log.d(serviceLogTag, "Loading Config")
        val preferences = getSharedPreferences(context)
        return preferences.getString("CONFIG", null)
    }

    private fun storeConfig(context: Context) {
        if (mService != null) {
            try {
                Log.d(serviceLogTag, "Saving Config")
                val config = mService!!.getConfig()
                val preferences = getSharedPreferences(context)
                preferences.edit().putString("CONFIG", config).apply()
            } catch (e: ServiceException) {
                Log.e(serviceLogTag, "Failed to store config: $e")
            } catch (e: java.lang.Exception) {
                Log.e(serviceLogTag, "Failed to store config: $e")
            }
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

    override fun notifyError(email: String, error: ServiceException) {
        Toast.makeText(this, serviceExceptionToErrorStr(error, email), Toast.LENGTH_SHORT).show()
        Log.e(serviceLogTag, "Failed to load '$email' from config: $error")
    }
}


fun getAppNameForBackend(backend: String): String? {
    return when (backend) {
        "Proton Mail" -> "ch.protonmail.android"

        else -> null
    }
}