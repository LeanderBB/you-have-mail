package dev.lbeernaert.youhavemail.service

import android.app.*
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.graphics.Color
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


const val serviceLogTag = "observer"


class ObserverService : Service(), Notifier, ServiceFromConfigCallback {
    private var wakeLock: PowerManager.WakeLock? = null
    private var isServiceStarted = false
    private val binder = LocalBinder()
    private val notificationChannelIdService = "YOU_HAVE_MAIL_SERVICE"
    private val notificationChannelIdAlerter = "YOU_HAVE_MAIL_NOTIFICATION"
    private val coroutineScope = CoroutineScope(
        Dispatchers.Default
    )
    private var _accountListFlow: MutableStateFlow<List<ObserverAccount>> =
        MutableStateFlow(ArrayList())
    val accountList: StateFlow<List<ObserverAccount>> get() = _accountListFlow


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
        if (mService != null) {
            try {
                val config = mService!!.getConfig()
                storeConfig(this, config)
            } catch (e: ServiceException) {
                Log.e(serviceLogTag, "Failed to store config: $e")
            } catch (e: java.lang.Exception) {
                Log.e(serviceLogTag, "Failed to store config: $e")
            }
        }
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

    fun clearInLoginAccount() {
        mInLoginAccount?.destroy()
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
                newService(this)
            } else {
                newServiceFromConfig(this, this, config)
            }
            mBackends.addAll(mService!!.getBackends())
            updateAccountList()
        } catch (e: ServiceException) {
            Log.e(serviceLogTag, "Failed to create service:$e")
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        coroutineScope.cancel()
        mInLoginAccount?.destroy()
        mBackends.forEach {
            it.destroy()
        }
        mBackends.clear()
        mService?.destroy()
        Log.d(serviceLogTag, "The service has been destroyed")
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
        val channelService = NotificationChannel(
            notificationChannelIdService,
            "You Have Mail Background Service",
            NotificationManager.IMPORTANCE_LOW
        ).let {
            it.description = "You Have Mail Background Service"
            it.enableLights(false)
            it.enableVibration(false)
            it
        }
        notificationManager.createNotificationChannel(channelService)
        val channelAlerter = NotificationChannel(
            notificationChannelIdAlerter,
            "You Have Mail Alerter",
            NotificationManager.IMPORTANCE_HIGH
        ).let {
            it.description = "You Have Mail Notifications"
            it.enableLights(true)
            it.lightColor = Color.WHITE
            it.enableVibration(true)
            it
        }
        notificationManager.createNotificationChannel(channelAlerter)
    }

    private fun createServiceNotification(): Notification {
        val pendingIntent: PendingIntent =
            Intent(this, MainActivity::class.java).let { notificationIntent ->
                PendingIntent.getActivity(this, 0, notificationIntent, PendingIntent.FLAG_IMMUTABLE)
            }

        val builder: Notification.Builder = Notification.Builder(
            this,
            notificationChannelIdService
        )

        return builder
            .setContentTitle("You Have Mail Service")
            .setContentText("You have Mail Background Service")
            .setContentIntent(pendingIntent)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setVisibility(Notification.VISIBILITY_SECRET)
            .setCategory(Notification.CATEGORY_SERVICE)
            .setOngoing(true)
            .setTicker("You Have Mail Service")
            .build()
    }

    private fun createAlertNotification(email: String, messageCount: UInt): Notification {
        val pendingIntent: PendingIntent =
            Intent(this, MainActivity::class.java).let { notificationIntent ->
                PendingIntent.getActivity(this, 0, notificationIntent, PendingIntent.FLAG_IMMUTABLE)
            }

        val builder: Notification.Builder = Notification.Builder(
            this,
            notificationChannelIdAlerter
        )

        return builder
            .setContentTitle("You Have Mail")
            .setContentText("$email has $messageCount new message(s)")
            .setContentIntent(pendingIntent)
            .setVisibility(Notification.VISIBILITY_SECRET)
            .setCategory(Notification.CATEGORY_STATUS)
            .setSmallIcon(R.mipmap.ic_launcher)
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
            notificationChannelIdAlerter
        )

        val errorString = serviceExceptionToErrorStr(err, email)

        return builder
            .setContentTitle("You Have Mail")
            .setContentText("$email error: $errorString")
            .setContentIntent(pendingIntent)
            .setVisibility(Notification.VISIBILITY_SECRET)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setTicker("You Have Mail Alert")
            .build()
    }

    private fun createAccountStatusNotification(text: String): Notification {
        val pendingIntent: PendingIntent =
            Intent(this, MainActivity::class.java).let { notificationIntent ->
                PendingIntent.getActivity(this, 0, notificationIntent, PendingIntent.FLAG_IMMUTABLE)
            }

        val builder: Notification.Builder = Notification.Builder(
            this,
            notificationChannelIdAlerter
        )

        return builder
            .setContentTitle("You Have Mail")
            .setContentText(text)
            .setContentIntent(pendingIntent)
            .setVisibility(Notification.VISIBILITY_SECRET)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setTicker("You Have Mail Alert")
            .build()
    }

    override fun newEmail(account: String, backend: String, count: UInt) {
        Log.d(serviceLogTag, "New Mail: $account ($backend) num=$count")
        val notification = createAlertNotification(account, count)
        with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            if (this.areNotificationsEnabled()) {
                notify(2, notification)
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
        with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            if (this.areNotificationsEnabled()) {
                notify(6, notification)
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
        with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            if (this.areNotificationsEnabled()) {
                notify(3, notification)
            }
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
        Log.d(serviceLogTag, "Loading Config")
        val preferences = getSharedPreferences(context)
        return preferences.getString("CONFIG", null)
    }

    private fun storeConfig(context: Context, config: String) {
        Log.d(serviceLogTag, "Saving Config")
        val preferences = getSharedPreferences(context)
        if (!preferences.edit().putString("CONFIG", config).commit()) {
            Log.e(serviceLogTag, "Failed to write config to disk")
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