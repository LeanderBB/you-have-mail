package dev.lbeernaert.youhavemail.service

import android.app.*
import android.app.Service
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.os.Binder
import android.os.IBinder
import android.os.PowerManager
import dev.lbeernaert.youhavemail.*;

class ObserverService : Service(), Notifier {
    private var wakeLock: PowerManager.WakeLock? = null
    private var isServiceStarted = false
    private val binder = LocalBinder()
    private val notificationChannelIdService = "YOU_HAVE_MAIL_SERVICE"
    private val notificationChannelIdAlerter = "YOU_HAVE_MAIL_NOTIFICATION"

    // Have to keep this here or it won't survive activity refreshes
    private var mInLoginAccount: Account? = null
    private var mBackends: ArrayList<Backend> = ArrayList()

    var mService: dev.lbeernaert.youhavemail.Service? = null

    inner class LocalBinder : Binder() {
        fun getService(): ObserverService = this@ObserverService
    }

    override fun onBind(intent: Intent?): IBinder? {
        Log.d("Some component wants to bind with the service")
        return binder
    }

    override fun onUnbind(intent: Intent?): Boolean {
        Log.d("Some component unbound from the system")
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
        Log.d("onStartCommand executed with startId: $startId")
        if (intent != null) {
            val action = intent.action
            Log.d("Using intent with action $action")
            when (action) {
                Actions.START.name -> startService()
                Actions.STOP.name -> stopService()
                else -> Log.e("Unknown action:  $action")
            }
        } else {
            Log.d("Null intent, probably restarted by the system")
        }

        return START_STICKY
    }

    override fun onCreate() {
        super.onCreate()

        createNotificationChannel()

        Log.d("Service has been created")
        startForeground(1, createServiceNotification())
        try {
            mService = newService(this)
            mBackends.addAll(mService!!.getBackends())
        } catch (e: ServiceException) {
            Log.e("Failed to create service:$e")
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        mInLoginAccount?.destroy()
        mBackends.forEach {
            it.destroy()
        }
        mBackends.clear()
        mService?.destroy()
        Log.d("The service has been destroyed")
    }

    private fun startService() {
        if (isServiceStarted) return
        Log.d("Starting foreground service task")

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
        Log.d("Stopping foreground service")
        try {
            wakeLock?.let {
                if (it.isHeld) {
                    it.release()
                }
            }
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
        } catch (e: java.lang.Exception) {
            Log.e("Service stopped without being started: ${e.message}")
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
            .setContentText("Email $email has $messageCount new message(s)")
            .setContentIntent(pendingIntent)
            .setVisibility(Notification.VISIBILITY_SECRET)
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

        return builder
            .setContentTitle("You Have Mail")
            .setContentText("Email $email has encountered an error ${err.message}")
            .setContentIntent(pendingIntent)
            .setVisibility(Notification.VISIBILITY_SECRET)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setTicker("You Have Mail Alert")
            .build()
    }

    override fun newEmail(account: String, backend: String, count: UInt) {
        val notification = createAlertNotification(account, count)
        with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            if (this.areNotificationsEnabled()) {
                notify(1, notification)
            }
        }
    }

    override fun accountAdded(email: String) {
    }

    override fun accountLoggedOut(email: String) {
    }

    override fun accountRemoved(email: String) {
    }

    override fun accountOffline(email: String) {
    }

    override fun accountOnline(email: String) {
    }

    override fun accountError(email: String, error: ServiceException) {
        val notification = createAccountErrorNotification(email, error)
        with(this.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            if (this.areNotificationsEnabled()) {
                notify(1, notification)
            }
        }
    }
}