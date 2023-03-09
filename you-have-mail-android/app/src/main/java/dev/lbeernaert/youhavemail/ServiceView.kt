package dev.lbeernaert.youhavemail

import dev.lbeernaert.youhavemail.service.ObserverService
import kotlinx.coroutines.launch

class ServiceView {
    private var mObserverService: ObserverService? = null
    private var mService: Service? = null
    private var mAccounts: ArrayList<ObserverAccount> = ArrayList()
    private var mBackends: ArrayList<Backend> = ArrayList()
    private var mRefreshAccounts = false

    fun setService(service: ObserverService) {
        mObserverService = service
        mService = mObserverService!!.mService
        mBackends.addAll(mService!!.getBackends())
        mAccounts.addAll(mService!!.getObservedAccounts())
    }

    fun removeService() {
        mService = null
        mBackends.forEach {
            it.destroy()
        }
        mBackends.clear()
        clearAccounts()
    }

    fun getAccounts(): List<ObserverAccount> {
        refreshAccounts();
        return mAccounts
    }

    fun getBackends(): List<Backend> {
        return mBackends
    }

    fun getService(): Service? {
        return mService
    }

    fun requiresAccountRefresh() {
        mRefreshAccounts = true
    }

    private fun clearAccounts() {
        mAccounts.forEach {
            // Uniffi-rs lifetime requirements
            it.destroy()
        }
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

    fun setInLoginAccount(account: Account) {
        mObserverService?.setInLoginAccount(account)
    }

    fun getInLoginAccount(): Account? {
        return mObserverService?.getInLoginAccount()
    }

    fun clearInLoginAccount() {
        mObserverService?.clearInLoginAccount()
    }

    fun runServiceRequest(req: (service: ObserverService) -> Unit) {
        return mObserverService!!.runServiceRequest(req)
    }
}