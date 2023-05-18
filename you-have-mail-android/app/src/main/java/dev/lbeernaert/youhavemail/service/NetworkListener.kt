package dev.lbeernaert.youhavemail.service

import android.net.ConnectivityManager.NetworkCallback
import android.net.Network

class NetworkListener(private val service: ObserverService) : NetworkCallback() {

    override fun onAvailable(network: Network) {
        super.onAvailable(network)
        service.acquireWakeLock()
        service.resumeService()
    }

    override fun onLost(network: Network) {
        super.onLost(network)
        service.pauseServiceNoNetwork()
        service.releaseWakeLock()
    }
}