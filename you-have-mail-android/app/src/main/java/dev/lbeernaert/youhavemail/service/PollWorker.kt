package dev.lbeernaert.youhavemail.service

import android.content.Context
import android.content.Intent
import android.util.Log
import androidx.localbroadcastmanager.content.LocalBroadcastManager
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequest
import androidx.work.WorkManager
import androidx.work.Worker
import androidx.work.WorkerParameters
import java.lang.Long.max
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

fun registerWorker(ctx: Context, minutes: Long) {
    WorkManager.getInstance(ctx).cancelAllWorkByTag(TAG)
    val minutes = max(15, minutes)

    Log.d(TAG, "Registering worker with $minutes min interval")

    val work =
        PeriodicWorkRequest.Builder(PollWorker::class.java, minutes, TimeUnit.MINUTES)
            .addTag(TAG)
            .setConstraints(
                Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()
            )
            .build()
    WorkManager.getInstance(ctx)
        .enqueueUniquePeriodicWork(
            POLL_WORKER_JOB_NAME,
            ExistingPeriodicWorkPolicy.CANCEL_AND_REENQUEUE,
            work
        )
}