package dev.lbeernaert.youhavemail.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.SharedPreferences
import android.util.Log
import androidx.localbroadcastmanager.content.LocalBroadcastManager
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import dev.lbeernaert.youhavemail.Account
import dev.lbeernaert.youhavemail.Backend
import dev.lbeernaert.youhavemail.Event
import dev.lbeernaert.youhavemail.Proxy
import dev.lbeernaert.youhavemail.Yhm
import dev.lbeernaert.youhavemail.activityLogTag
import dev.lbeernaert.youhavemail.newEncryptionKey
import dev.lbeernaert.youhavemail.yhmLogError
import dev.lbeernaert.youhavemail.yhmLogInfo
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import java.io.File

/*
data class ServiceAccount(
    val email: String,
    val backend: String,
    val state: MutableStateFlow<ObserverAccountStatus>,
    val proxy: MutableStateFlow<Proxy?>
)

class ObserverServiceState {
    private var mAccounts: ArrayList<ServiceAccount> = ArrayList()
    private var mAccountsFlow: MutableStateFlow<List<ServiceAccount>> =
        MutableStateFlow(ArrayList())
 */

const val STATE_LOG_TAG = "state"

// Has to be global singleton for now so that the ids are accessible for the worker
// and the system. Could be moved to a shared preferences setup for persistent
// changes.
var NOTIFICATION_STATE = NotificationState()

class State(context: Context) : BroadcastReceiver() {
    private var mPollInterval = MutableStateFlow(15UL)
    private var mYhm: Yhm
    private var mAccounts: MutableStateFlow<List<Account>>
    private var mOpenAccount: MutableStateFlow<Account?> = MutableStateFlow(null)
    var mLoginSequence: LoginSequence? = null

    init {
        val key = getOrCreateEncryptionKey(context)
        val dbPath = getDatabasePath(context)
        mYhm = Yhm(dbPath, encryptionKey = key)
        mAccounts = MutableStateFlow(mYhm.accounts())
        val pollInterval = mYhm.pollInterval()
        mPollInterval.value = pollInterval
        registerWorker(context, pollInterval.toLong() / 60, false)

        val filter = IntentFilter()
        filter.addAction(POLL_INTENT)
        LocalBroadcastManager.getInstance(context)
            .registerReceiver(this, filter)
    }


    fun migrateAccounts(context: Context) {
        val file = getConfigFilePathV1(context)
        if (!file.exists()) {
            return
        }
        try {
            yhmLogInfo("Found v1 config, importing...")
            mYhm.importV1(file.path)
            yhmLogInfo("Found v1 config, importing...Done")
        } catch (e: Exception) {
            yhmLogError("Failed to migrate: $e")
            try {
                createAndDisplayServiceErrorNotification(context, "Failed to migrate accounts: $e")
            } catch (e: Exception) {
                Log.e(activityLogTag, "Failed to create exception");
            }
        } finally {
            try {
                file.delete()
            } catch (e: Exception) {
                yhmLogError("Failed to delete old config file: $e")
            }
        }

        refreshData()
    }

    /**
     * Update poll interval.
     *
     */
    fun setPollInterval(context: Context, intervalSeconds: ULong) {
        Log.i(STATE_LOG_TAG, "Interval $intervalSeconds Seconds")
        mYhm.setPollInterval(intervalSeconds)
        mPollInterval.value = intervalSeconds
        registerWorker(context, (intervalSeconds.toLong() / 60), true)
    }

    /**
     * Get the current poll interval
     */
    fun getPollInterval(): StateFlow<ULong> {
        return mPollInterval
    }

    /**
     * Get list of known accounts
     */
    fun accounts(): StateFlow<List<Account>> {
        return mAccounts
    }

    /**
     * Get list of backends.
     */
    fun backends(): List<Backend> {
        return mYhm.backends()
    }

    /**
     * Open an account fo detailed inspection
     */
    fun account(email: String): Account? {
        return mYhm.account(email)
    }

    /**
     * Call this function when a new account has been added.
     */
    fun onAccountAdded() {
        refreshData()
    }

    /**
     * Logout account by email.
     */
    fun logout(email: String) {
        mYhm.logout(email)
        refreshData()
    }

    /**
     * Delete an account by email.
     */
    fun delete(email: String) {
        mYhm.delete(email)
        refreshData()
    }

    fun yhm(): Yhm {
        return mYhm
    }

    /**
     * Create a new login sequence for a given backend.
     */
    fun newLoginSequence(backendName: String, proxy: Proxy?): LoginSequence {
        if (backendName == PROTON_BACKEND_NAME) {
            return ProtonLogin(this, proxy)
        }

        throw RuntimeException("Unknown backend")
    }

    /**
     * Unregister receiver and release resource.
     */
    fun close(context: Context) {
        Log.i(STATE_LOG_TAG, "Closing")
        LocalBroadcastManager.getInstance(context).unregisterReceiver(this)
        mAccounts.value = ArrayList()
        mLoginSequence = null
        mYhm.close()
    }

    // Refresh internal state from the database.
    private fun refreshData() {
        val accounts = mYhm.accounts()
        if (mOpenAccount.value != null) {
            val accountEmail = mOpenAccount.value!!.email()
            mOpenAccount.value = accounts.find {
                it.email() == accountEmail
            }
        }
        mAccounts.value = accounts
    }

    override fun onReceive(context: Context?, intent: Intent?) {
        if (intent == null) {
            return
        }
        if (intent.action == null) {
            return
        }

        if (context == null) {
            Log.e("STATE", "No context")
            return
        }

        when (intent.action) {
            POLL_INTENT -> {
                Log.d(STATE_LOG_TAG, "Received poll intent")
                refreshData()
            }
        }
    }
}

/**
 *  Get configuration file path.
 */
private fun getConfigFilePathV1(context: Context): File {
    return File(context.filesDir.canonicalPath, "config_v2")
}

/**
 *  Get database path.
 */
fun getDatabasePath(context: Context): String {
    return context.filesDir.canonicalPath + "/yhm.db"
}

/**
 * Get log path.
 */
fun getLogPath(context: Context): File {
    return File(context.filesDir.canonicalPath, "logs")
}

/**
 * Load encryption key for the application.
 */
fun getOrCreateEncryptionKey(context: Context): String {
    Log.d(STATE_LOG_TAG, "Loading Encryption key")
    val preferences = getSharedPreferences(context)
    val key = preferences.getString("KEY", null)
    return if (key == null) {
        Log.d(STATE_LOG_TAG, "No key exists, creating new key")
        val newKey = newEncryptionKey()
        preferences.edit().putString("KEY", newKey).apply()
        newKey
    } else {
        key
    }
}

/**
 * Get the encrypted shared preferences for this application.
 */
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

/**
 * Get the application we should open based on the backend name.
 */
fun getAppNameForBackend(backend: String): String? {
    return when (backend) {
        "Proton Mail" -> "ch.protonmail.android"
        "Proton Mail V-Other" -> "ch.protonmail.android"
        "Null Backend" -> "ch.protonmail.android"

        else -> null
    }
}

fun accountStatusString(account: Account): String {
    var statusString = ""
    if (account.isLoggedOut()) {
        statusString = "Logged Out"
    } else {
        val last_poll = account.lastPoll();
        if (last_poll == null) {
            statusString = "Not Polled"
        } else {
            val last_event = account.lastEvent();
            if (last_event != null) {
                var result = "Polled"
                when (last_event) {
                    is Event.Error -> {
                        result = "Error"
                    }

                    is Event.Offline -> {
                        result = "Offline"
                    }

                    else -> {

                    }
                }
                statusString = "$result (${last_poll})"
            }
        }
    }

    return statusString
}