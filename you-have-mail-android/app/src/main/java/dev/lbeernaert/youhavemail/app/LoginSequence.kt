package dev.lbeernaert.youhavemail.app

import androidx.navigation.NavHostController
import dev.lbeernaert.youhavemail.ProtonLoginException
import dev.lbeernaert.youhavemail.ProtonLoginSequence
import dev.lbeernaert.youhavemail.Proxy
import dev.lbeernaert.youhavemail.Yhm
import dev.lbeernaert.youhavemail.screens.Routes

/**
 * Minor abstraction over actual login process for each backend. The backend does not need
 * to implement all these methods and backend specific methods can be added as needed.
 */
interface LoginSequence {
    /**
     * Perform login and navigate to the next screen.
     */
    fun login(email: String, password: String)

    /**
     * Submit totp code and navigate to the next screen.
     */
    fun totp(code: String)

    /**
     * Proton specific captcha handler.
     *
     * Submit the token and navigate to the next screen.
     */
    fun protonCaptcha(token: String)


    /**
     * Proton specific, get captcha html.
     */
    fun protonCaptchaHtml(): String

    /**
     * Advance to the next stage in the login sequence.
     */
    fun next(navController: NavHostController)

    /**
     * Return the backend name for this login sequence.
     */
    fun backendName(): String
}

/**
 * Proton Login Sequence implementation
 */
class ProtonLogin(state: State, proxy: Proxy?) : LoginSequence {
    private var mState = state
    private var mSequence = ProtonLoginSequence(proxy)

    // Email is recorded for auto captcha login
    private var mEmail: String = ""

    // Password is recorded for auto captcha login
    private var mPassword: String = ""
    private var mHumanVerificationData: String? = null
    private var mCaptchaHtml: String? = null

    override fun login(email: String, password: String) {
        mEmail = email;
        mPassword = password
        try {
            mSequence.login(email, password, mHumanVerificationData)
        } catch (e: ProtonLoginException) {
            handelLoginException(e)
        }
    }

    override fun totp(code: String) {
        try {
            mSequence.submitTotp(code)
        } catch (e: ProtonLoginException) {
            handelLoginException(e)
        }
    }

    override fun protonCaptcha(token: String) {
        mHumanVerificationData = token
        mCaptchaHtml = null
        login(email = mEmail, password = mPassword)
    }

    override fun protonCaptchaHtml(): String {
        return mCaptchaHtml.orEmpty()
    }

    override fun next(navController: NavHostController) {
        if (mCaptchaHtml != null) {
            navController.navigate(Routes.ProtonCaptcha.route)
        } else if (mSequence.isAwaitingTotp()) {
            navController.navigate(Routes.TOTP.route)
        } else if (mSequence.isLoggedIn()) {
            mSequence.createAccount(mState.yhm())
            navController.popBackStack(Routes.Main.route, false)
        }
    }

    override fun backendName(): String {
        return PROTON_BACKEND_NAME
    }

    private fun handelLoginException(e: ProtonLoginException) {
        when (e) {
            is ProtonLoginException.HumanVerificationRequired -> {
                // Avoid Loop
                if (mHumanVerificationData != null) {
                    throw RuntimeException("Captcha Request Loop")
                }
                mCaptchaHtml = mSequence.captcha(e.v1.token)
            }

            else -> {
                throw e
            }
        }
    }
}