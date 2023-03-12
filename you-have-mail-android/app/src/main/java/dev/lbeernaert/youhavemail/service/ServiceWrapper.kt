package dev.lbeernaert.youhavemail.service

import dev.lbeernaert.youhavemail.Account
import dev.lbeernaert.youhavemail.Backend
import dev.lbeernaert.youhavemail.ObserverAccount
import dev.lbeernaert.youhavemail.Service

class ServiceWrapper {
    private var mObserverService: ObserverService? = null
    private var mService: Service? = null
    private var mAccounts: ArrayList<ObserverAccount> = ArrayList()
    private var mRefreshAccounts = false

    fun setService(service: ObserverService) {
        mObserverService = service
        mService = mObserverService!!.mService
        mAccounts.addAll(mService!!.getObservedAccounts())
    }

    fun removeService() {
        mObserverService = null
        mService = null
        clearAccounts()
    }

    fun getAccounts(): List<ObserverAccount> {
        refreshAccounts();
        return mAccounts
    }

    fun getBackends(): List<Backend> {
        return mObserverService!!.getBackends()
    }

    private fun clearAccounts() {
        mAccounts.clear()
    }

    private fun refreshAccounts() {
        if (mRefreshAccounts) {
            clearAccounts()

            if (mService != null) {
                mAccounts.addAll(mService!!.getObservedAccounts())
            }

            mRefreshAccounts = false
        }
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
        mRefreshAccounts = true
    }

    fun logoutAccount(email: String) {
        mService!!.logoutAccount(email)
        mRefreshAccounts = true
    }

    fun removeAccount(email: String) {
        mService!!.removeAccount(email)
        mRefreshAccounts = true
    }

    fun backendIndexByName(name: String): Int? {
        for (b in getBackends().listIterator().withIndex()) {
            if (b.value.name() == name) {
                return b.index
            }
        }

        return null
    }
}