package dev.lbeernaert.youhavemail.service

import android.util.Log
import dev.lbeernaert.youhavemail.*
import kotlinx.coroutines.flow.StateFlow

class ServiceWrapper {
    private var mObserverService: ObserverService? = null
    private var mService: Service? = null

    fun isReady(): Boolean {
        return mObserverService != null
    }

    fun setService(service: ObserverService) {
        mObserverService = service
        mService = mObserverService!!.mService
    }

    fun removeService() {
        mObserverService = null
        mService = null
    }

    fun getAccountsStateFlow(): StateFlow<List<ObserverAccount>> {
        return mObserverService!!.accountList
    }

    fun getPollIntervalValueStateFlow(): StateFlow<ULong> {
        return mObserverService!!.pollInterval
    }

    fun getBackends(): List<Backend> {
        return mObserverService!!.getBackends()
    }

    fun getInLoginAccount(): Account? {
        return mObserverService?.getInLoginAccount()
    }

    fun getInLoginProxy(): Proxy? {
        return mObserverService!!.getInLoginProxy()
    }

    fun setInLoginProxy(proxy: Proxy?) {
        return mObserverService!!.setInLoginProxy(proxy)
    }

    fun newAccount(backend: Backend, email: String): Account {
        val account = mService!!.newAccount(backend, email, mObserverService!!.getInLoginProxy())
        mObserverService!!.setInLoginAccount(account)
        return account
    }

    fun addAccount(account: Account) {
        mService!!.addAccount(account)
    }

    fun logoutAccount(email: String) {
        mService!!.logoutAccount(email)
    }

    fun removeAccount(email: String) {
        mService!!.removeAccount(email)
    }

    fun checkProxy(backendIndex: Int, proxy: Proxy?) {
        if (proxy == null) {
            return
        }

        mService!!.checkProxy(getBackends()[backendIndex], proxy)
    }

    fun setAccountProxy(email: String, proxy: Proxy?) {
        mObserverService!!.setAccountProxy(email, proxy)
    }

    fun backendIndexByName(name: String): Int? {
        for (b in getBackends().listIterator().withIndex()) {
            if (b.value.name() == name) {
                return b.index
            }
        }

        return null
    }

    fun getAccount(index: Int): ObserverAccount? {
        try {
            val accounts = mObserverService!!.accountList.value
            if (index < accounts.size) {
                return accounts[index]
            }

            return null
        } catch (e: ServiceException) {
            Log.e(serviceLogTag, "Failed to get account by index: $e")
            return null
        }
    }

    fun setPollInterval(intervalSeconds: ULong) {
        mObserverService!!.setPollInterval(intervalSeconds)
    }

    fun getAccountActivity(email: String): List<AccountActivity> {
        return mObserverService!!.getAccountActivity(email)
    }
}