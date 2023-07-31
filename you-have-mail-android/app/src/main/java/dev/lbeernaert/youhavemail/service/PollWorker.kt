package dev.lbeernaert.youhavemail.service

import android.content.Context
import android.content.Intent
import android.util.Log
import androidx.localbroadcastmanager.content.LocalBroadcastManager
import androidx.work.Constraints
import androidx.work.Data
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequest
import androidx.work.PeriodicWorkRequest
import androidx.work.WorkManager
import androidx.work.Worker
import androidx.work.WorkerParameters
import java.util.concurrent.TimeUnit

private const val TAG = "PollWorker"
const val POLL_WORKER_JOB_NAME = "PollWorker"
const val POLL_INTENT_NAME = "POLL_INTENT"

class PollWorker(ctx: Context, params: WorkerParameters) :
    Worker(ctx, params) {

    override fun doWork(): Result {
        return try {
            val localIntent = Intent(POLL_INTENT_NAME)
            LocalBroadcastManager.getInstance(applicationContext).sendBroadcast(localIntent)
            Result.success()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to send local broadcast: $e")
            Result.failure()
        }
    }
}

class OneTimePollWorker(ctx: Context, params: WorkerParameters) :
    Worker(ctx, params) {

    override fun doWork(): Result {
        return try {
            val localIntent = Intent(POLL_INTENT_NAME)
            LocalBroadcastManager.getInstance(applicationContext).sendBroadcast(localIntent)

            registerWorker(applicationContext, inputData.getLong("INTERVAL", 15 * 60), false)

            Result.success()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to send local broadcast: $e")
            Result.failure()
        }
    }
}

fun registerWorker(ctx: Context, minutes: Long, cancel: Boolean) {
    val inputData = Data.Builder().putLong("INTERVAL", minutes).build()
    val constraints = Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()
    val wm = WorkManager.getInstance(ctx)

    if (cancel) {
        wm.cancelAllWorkByTag(TAG)
    }

    if (minutes >= 15) {
        Log.d(TAG, "Registering Periodic work with $minutes min interval")

        val work =
            PeriodicWorkRequest.Builder(PollWorker::class.java, minutes, TimeUnit.MINUTES)
                .addTag(TAG)
                .setInputData(inputData)
                .setConstraints(constraints)
                .build()
        wm
            .enqueueUniquePeriodicWork(
                POLL_WORKER_JOB_NAME,
                ExistingPeriodicWorkPolicy.CANCEL_AND_REENQUEUE,
                work
            )
    } else {
        Log.d(TAG, "Registering One Time work with $minutes min interval")
        val work = OneTimeWorkRequest.Builder(OneTimePollWorker::class.java).addTag(TAG)
            .setInputData(inputData).setConstraints(constraints)
            .setInitialDelay(minutes, TimeUnit.MINUTES)
            .build()

        wm.enqueue(work)
    }
}