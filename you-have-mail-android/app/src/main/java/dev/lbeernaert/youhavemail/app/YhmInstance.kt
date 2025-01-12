package dev.lbeernaert.youhavemail.app

import android.content.Context
import dev.lbeernaert.youhavemail.Yhm

/**
 * Yhm Singleton.
 */
class YhmInstance private constructor(var yhm: Yhm) {
    companion object {
        @Volatile
        private var instance: YhmInstance? = null

        /**
         * Get or create a Yhm instance.
         *
         * Throws exception on failure.
         */
        fun get(context: Context): YhmInstance {
            if (instance != null) {
                return instance!!
            }
            synchronized(this) {
                if (instance == null) {
                    val key = getOrCreateEncryptionKey(context)
                    val dbPath = getDatabasePath(context)
                    instance = YhmInstance(Yhm(dbPath, encryptionKey = key))
                }

                return instance!!
            }
        }
    }


}