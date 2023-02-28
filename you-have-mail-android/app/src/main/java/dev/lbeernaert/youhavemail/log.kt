package dev.lbeernaert.youhavemail

import android.util.Log

class Log {
    companion object {
        fun d(msg: String) {
            Log.d("you-have-mail", msg)
        }

        fun i(msg:String) {
            Log.i("you-have-mail", msg)
        }

        fun e(msg:String) {
            Log.e("you-have-mail", msg)
        }
    }
}