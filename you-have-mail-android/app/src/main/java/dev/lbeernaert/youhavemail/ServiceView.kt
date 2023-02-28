package dev.lbeernaert.youhavemail

import androidx.lifecycle.ViewModel

class ServiceView : ViewModel() {
    private var mService: Service? = null
    private var mAccounts: ArrayList<ObserverAccount> = ArrayList()
    private val mBackends: ArrayList<Backend> = ArrayList()

    fun setService(service: Service) {
        mService = service
        mBackends.addAll(mService!!.getBackends())
        mAccounts.addAll(mService!!.getObservedAccounts())
    }

    fun removeService() {
        mService = null
        mBackends.forEach {
            it.destroy()
        }
        mBackends.clear()
        mAccounts.forEach {
            it.destroy()
        }
        mAccounts.clear()
    }

    fun getAccounts(): List<ObserverAccount> {
        return mAccounts
    }

    fun getBackend(): List<Backend> {
        return mBackends
    }

    fun getService(): Service? {
        return mService
    }

}