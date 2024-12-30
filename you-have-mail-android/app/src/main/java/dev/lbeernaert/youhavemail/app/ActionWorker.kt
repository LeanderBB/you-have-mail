package dev.lbeernaert.youhavemail.app

import android.content.Context
import android.os.Handler
import android.util.Log
import android.widget.Toast
import androidx.work.Constraints
import androidx.work.Data
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequest
import androidx.work.WorkManager
import androidx.work.Worker
import androidx.work.WorkerParameters
import dev.lbeernaert.youhavemail.Yhm

private const val TAG = "ActionWorker"
private const val EmailKey = "Email"
private const val ActionKey = "Key"
private const val ActionDescKey = "ActionDesc"

/**
 * Worker which execute an action
 */
class ActionWorker(ctx: Context, params: WorkerParameters) :
    Worker(ctx, params) {

    override fun doWork(): Result {
        val email = this.inputData.getString(EmailKey)
        val action = this.inputData.getString(ActionKey)
        if (email == null || action == null) {
            return Result.failure()
        }
        val actionDesc = this.inputData.getString(ActionDescKey)
        Log.i(TAG, "email=$email action=$action")
        val handler = Handler(applicationContext.mainLooper);
        return try {
            executeAction(applicationContext, email, action)
            handler.postDelayed({
                Toast.makeText(
                    applicationContext,
                    "$actionDesc success",
                    Toast.LENGTH_LONG
                )
                    .show()
            }, 1000)
            Result.success()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to apply action: $e")
            handler.postDelayed({
                Toast.makeText(
                    applicationContext,
                    "$actionDesc failed: $e",
                    Toast.LENGTH_LONG
                )
                    .show()
            }, 1000)
            Result.failure()
        }
    }

    companion object {
        fun queue(context: Context, email: String, action: String, description: String) {
            val constraint =
                Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()
            val wm = WorkManager.getInstance(context)

            val inputData =
                Data.Builder().putString(EmailKey, email).putString(ActionKey, action).putString(
                    ActionDescKey, description
                ).build()

            val work = OneTimeWorkRequest.Builder(ActionWorker::class.java)
                .setInputData(inputData).setConstraints(constraint)
                .build()
            wm.enqueue(work)
        }
    }
}

private fun executeAction(context: Context, email: String, action: String) {
    val key = getOrCreateEncryptionKey(context)
    val dbPath = getDatabasePath(context)
    val yhm = Yhm.withoutDbInit(dbPath, encryptionKey = key)
    yhm.applyAction(email, action)
}