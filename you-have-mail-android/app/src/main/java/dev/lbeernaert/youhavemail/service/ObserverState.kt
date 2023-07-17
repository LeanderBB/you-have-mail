package dev.lbeernaert.youhavemail.service

import dev.lbeernaert.youhavemail.ObserverAccount
import dev.lbeernaert.youhavemail.ObserverAccountStatus
import dev.lbeernaert.youhavemail.Proxy
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow


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
    private var mPollInterval = MutableStateFlow(15UL)


    fun setFrom(accounts: List<ObserverAccount>) {
        mAccounts.clear()
        mAccounts = accounts.map {
            ServiceAccount(
                email = it.email,
                backend = it.backend,
                state = MutableStateFlow(it.status),
                proxy = MutableStateFlow(it.proxy)
            )
        }.sortedBy { serviceAccount -> serviceAccount.email }.toCollection(mAccounts)
        mAccountsFlow.value = mAccounts
    }

    fun onAccountAdded(email: String, backend: String, proxy: Proxy?) {
        val newAccount = ServiceAccount(
            email,
            backend = backend,
            state = MutableStateFlow(ObserverAccountStatus.ONLINE),
            proxy = MutableStateFlow(proxy)
        )
        mAccounts.add(newAccount)
        mAccounts.sortedBy { serviceAccount -> serviceAccount.email }
        mAccountsFlow.value = mAccounts
    }

    fun onAccountRemoved(email: String) {
        if (mAccounts.removeIf {
                it.email == email
            }) {
            mAccountsFlow.value = mAccounts
        }
    }

    fun onAccountOnline(email: String) {
        val account = mAccounts.find { it.email == email }
        if (account != null) {
            account.state.value = ObserverAccountStatus.ONLINE
        }
    }

    fun onAccountOffline(email: String) {
        val account = mAccounts.find { it.email == email }
        if (account != null) {
            account.state.value = ObserverAccountStatus.OFFLINE
        }
    }

    fun onAccountLoggedOut(email: String) {
        val account = mAccounts.find { it.email == email }
        if (account != null) {
            account.state.value = ObserverAccountStatus.LOGGED_OUT
        }
    }

    fun onAccountProxyChanged(email: String, proxy: Proxy?) {
        val account = mAccounts.find { it.email == email }
        if (account != null) {
            account.proxy.value = proxy
        }
    }

    fun getAccounts(): StateFlow<List<ServiceAccount>> {
        return mAccountsFlow
    }

    fun onPollIntervalChanged(interval: ULong) {
        mPollInterval.value = interval
    }

    fun getPollInterval(): StateFlow<ULong> {
        return mPollInterval
    }

    fun onNetworkLost() {
        for (account in mAccounts) {
            if (account.state.value != ObserverAccountStatus.LOGGED_OUT) {
                account.state.value = ObserverAccountStatus.OFFLINE
            }
        }
    }

    fun onNetworkRestored() {
        for (account in mAccounts) {
            if (account.state.value == ObserverAccountStatus.OFFLINE) {
                account.state.value = ObserverAccountStatus.ONLINE
            }
        }
    }
}