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

    fun getBackends(): List<Backend> {
        return mObserverService!!.getBackends()
    }

    fun getInLoginAccount(): Account? {
        return mObserverService?.getInLoginAccount()
    }

    fun newAccount(backend: Backend, email: String): Account {
        val account = mService!!.newAccount(backend, email)
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
            val accounts = mService!!.getObservedAccounts()
            if (index < accounts.size) {
                return accounts[index]
            }

            return null
        } catch (e: ServiceException) {
            Log.e(serviceLogTag, "Failed to get account by index: $e")
            return null
        }
    }
}