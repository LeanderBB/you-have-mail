package dev.lbeernaert.youhavemail.service

import android.app.*
import android.app.Service
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.os.Binder
import android.os.IBinder
import android.os.PowerManager
import dev.lbeernaert.youhavemail.*
import kotlinx.coroutines.*

class ObserverService : Service(), Notifier {
    private var wakeLock: PowerManager.WakeLock? = null
    private var isServiceStarted = false
    private val binder = LocalBinder()
    private val notificationChannelId = "YOU_HAVE_MAIL_SERVICE"
    private val coroutineScope = CoroutineScope(
        SupervisorJob() + Dispatchers.IO
    )

    // Have to keep this here or it won't survive activity refreshes
    private var mInLoginAccount: Account? = null

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
        } catch (e: ServiceException) {
            Log.e("Failed to create service:$e")
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        mService?.destroy()
        mInLoginAccount?.destroy()
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
        val channel = NotificationChannel(
            notificationChannelId,
            "You Have Mail Background Service",
            NotificationManager.IMPORTANCE_HIGH
        ).let {
            it.description = "You Have Mail Background Notifications"
            it.enableLights(true)
            it.lightColor = Color.RED
            it.enableVibration(true)
            it
        }
        notificationManager.createNotificationChannel(channel)
    }

    private fun createServiceNotification(): Notification {
        val pendingIntent: PendingIntent =
            Intent(this, MainActivity::class.java).let { notificationIntent ->
                PendingIntent.getActivity(this, 0, notificationIntent, PendingIntent.FLAG_IMMUTABLE)
            }

        val builder: Notification.Builder = Notification.Builder(
            this,
            notificationChannelId
        )

        return builder
            .setContentTitle("You Have Mail Service")
            .setContentText("This is your favorite endless service working")
            .setContentIntent(pendingIntent)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setTicker("Ticker text")
            .build()
    }

    override fun notify(email: String, messageCount: ULong) {
        Log.i("Email $email has $messageCount new message(s)")
    }

    override fun notifyError(email: String, error: ServiceException) {
        Log.e("Email $email suffered error:$error")
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

    fun runServiceRequest(req: suspend (service: ObserverService) -> Unit) {
        val self = this
        coroutineScope.launch {
            req(self)
        }
    }
}