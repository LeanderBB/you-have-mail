package dev.lbeernaert.youhavemail.app

import android.content.Context
import android.content.Intent
import android.util.Log
import androidx.localbroadcastmanager.content.LocalBroadcastManager
import androidx.work.Constraints
import androidx.work.Data
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequest
import androidx.work.PeriodicWorkRequest
import androidx.work.WorkManager
import androidx.work.Worker
import androidx.work.WorkerParameters
import androidx.work.hasKeyWithValueOfType
import dev.lbeernaert.youhavemail.Event
import dev.lbeernaert.youhavemail.Yhm
import dev.lbeernaert.youhavemail.YhmException
import dev.lbeernaert.youhavemail.initLog
import dev.lbeernaert.youhavemail.yhmLogError
import dev.lbeernaert.youhavemail.yhmLogInfo
import java.util.concurrent.TimeUnit

private const val TAG = "PollWorker"
private const val TAG_ONE_SHOT = "OneShotWorker"
private const val POLL_WORKER_JOB_NAME = "PollWorker"
private const val ONE_SHOT_WORKER_JOB_NAME = "OneShotWorker"
const val POLL_INTENT = "POLL_INTENT"
const val POLL_ERROR_KEY = "POLL_ERROR"


/**
 * Periodic Worker which polls at longer intervals.
 */
class PollWorker(ctx: Context, params: WorkerParameters) :
    Worker(ctx, params) {

    override fun doWork(): Result {
        return try {
            poll(applicationContext)
            Result.success()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to send local broadcast: $e")
            Result.failure()
        }
    }
}

/**
 * One time poll worker for shorter intervals.
 *
 * If no input data is specified for the interval, subsequent launches are not repeated.
 */
class OneTimePollWorker(ctx: Context, params: WorkerParameters) :
    Worker(ctx, params) {

    override fun doWork(): Result {
        try {
            poll(applicationContext)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to poll: $e")
            yhmLogError("Unhandled exception: $e")
            return Result.failure()
        }

        try {
            if (inputData.hasKeyWithValueOfType<Long>("INTERVAL")) {
                val interval = inputData.getLong("INTERVAL", 15)
                registerWorker(applicationContext, interval, false)
            }
        } catch (e: Exception) {
            yhmLogError("Failed to re-register one time poll worker: $e")
            createServiceErrorNotification(applicationContext, "Failed to create worker: $e")
        }

        return Result.success()
    }
}

/**
 * Poll all accounts.
 */
private fun poll(context: Context) {
    val key = getOrCreateEncryptionKey(context)
    val dbPath = getDatabasePath(context)
    val yhm: Yhm
    try {
        yhm = Yhm.withoutDbInit(dbPath, encryptionKey = key)
    } catch (e: YhmException) {
        createServiceErrorNotification(context, "Failed to Create Yhm", e)
        return
    }

    var error: String? = null
    try {
        yhm.poll()
        try {
            val events = yhm.lastEvents()
            for (event in events) {
                when (event) {
                    is Event.Email -> {
                        for (email in event.emails) {
                            NOTIFICATION_STATE.onNewEmail(
                                context,
                                event.email,
                                event.backend,
                                email,
                            )
                        }
                    }

                    is Event.Error -> {
                        NOTIFICATION_STATE.onError(context, event.v1, event.v2)
                    }

                    is Event.LoggedOut -> {
                        NOTIFICATION_STATE.onLoggedOut(
                            context,
                            event.v1,
                        )
                    }

                    is Event.Offline -> {
                        // Do nothing.
                    }
                }
            }
        } catch (e: Exception) {
            createServiceErrorNotification(context, "Failed to retrieve events: $e")
        }
    } catch (e: YhmException) {
        error = e.toString()
        createServiceErrorNotification(context, error)
    } finally {
        yhm.close()
    }


    // While it would be preferable to have a watcher into the database detect, these changes and
    // since currently there exists no such thing for rust, we simply broad cast the success of
    // this work and let the main activity handle the notification state.

    try {
        val localIntent = Intent(POLL_INTENT)
        localIntent.putExtra(POLL_ERROR_KEY, error)
        LocalBroadcastManager.getInstance(context).sendBroadcast(localIntent)
    } catch (e: Exception) {
        yhmLogError("Failed to publish broadcast: $e")
        createServiceErrorNotification(context, "Failed to publish broadcast: $e")
    }
}

/**
 * Get worker constraints
 */
private fun constraints(): Constraints {
    return Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()
}

fun registerWorker(ctx: Context, minutes: Long, cancel: Boolean) {
    initLog(getLogPath(ctx).path)
    val inputData = Data.Builder().putLong("INTERVAL", minutes).build()
    val constraints = constraints()
    val wm = WorkManager.getInstance(ctx)

    if (cancel) {
        wm.cancelAllWorkByTag(TAG)
    }

    if (minutes >= 15) {
        Log.d(TAG, "Registering Periodic work with $minutes minutes interval")

        val work =
            PeriodicWorkRequest.Builder(PollWorker::class.java, minutes, TimeUnit.MINUTES)
                .addTag(TAG)
                .setInputData(inputData)
                .setConstraints(constraints)
                .build()
        wm.enqueueUniquePeriodicWork(
            POLL_WORKER_JOB_NAME,
            ExistingPeriodicWorkPolicy.KEEP,
            work
        )
    } else {
        Log.d(TAG, "Registering One Time work with $minutes minutes interval")
        val work = OneTimeWorkRequest.Builder(OneTimePollWorker::class.java).addTag(TAG)
            .setInputData(inputData).setConstraints(constraints)
            .setInitialDelay(minutes, TimeUnit.MINUTES)
            .build()

        wm.enqueueUniqueWork(
            POLL_WORKER_JOB_NAME,
            ExistingWorkPolicy.REPLACE,
            work
        )
    }
}

/**
 * Register a poll job that only runs once.
 */
fun oneshotWorker(ctx: Context) {
    Log.d(TAG, "Registering one time shot worker")
    val work = OneTimeWorkRequest.Builder(OneTimePollWorker::class.java).addTag(TAG_ONE_SHOT)
        .setConstraints(constraints())
        .build()

    val wm = WorkManager.getInstance(ctx)
    wm.enqueueUniqueWork(ONE_SHOT_WORKER_JOB_NAME, ExistingWorkPolicy.KEEP, work)
}