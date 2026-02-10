package dev.lbeernaert.youhavemail.app

import dev.lbeernaert.youhavemail.yhmLogInfo
import java.util.concurrent.atomic.AtomicInteger

class ScreenshotMode {
    companion object {
        fun isEnabled() : Boolean {
            return SCREENSHOT_MODE_ENABLED.get() != 0
        }

        fun enable(value: Boolean)  {
            yhmLogInfo("Screenshot mode $value")
           if (value)  {
               SCREENSHOT_MODE_ENABLED.set(1)
           } else {
               SCREENSHOT_MODE_ENABLED.set(0)
           }
        }

        fun redact(str:String) : String {
            return if (isEnabled()) {
               "[REDACTED]"
            } else {
                str
            }
        }
    }
}

private var SCREENSHOT_MODE_ENABLED: AtomicInteger = AtomicInteger(0)