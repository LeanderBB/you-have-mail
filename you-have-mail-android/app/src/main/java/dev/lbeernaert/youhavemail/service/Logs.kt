package dev.lbeernaert.youhavemail.service

import android.content.Context
import android.net.Uri
import android.util.Log
import android.widget.Toast
import dev.lbeernaert.youhavemail.R
import java.io.BufferedOutputStream
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

const val logsExportLogTag = "logs-export"
const val LOG_EXPORT_REQUEST = 1024

fun exportLogs(context: Context, outputFile: Uri) {
    try {
        val inputDirectory = getLogPath(context)
        Log.d(logsExportLogTag, "Preparing log zip file: $outputFile")
        ZipOutputStream(
            BufferedOutputStream(
                context.contentResolver.openOutputStream(outputFile)
            )
        ).use { zos ->
            inputDirectory.walkTopDown().forEach { file ->
                val zipFileName =
                    file.absolutePath.removePrefix(inputDirectory.absolutePath).removePrefix("/")
                val entry = ZipEntry("$zipFileName${(if (file.isDirectory) "/" else "")}")
                zos.putNextEntry(entry)
                if (file.isFile) {
                    Log.d(logsExportLogTag, "Adding file to zip: $zipFileName - $file")
                    file.inputStream().use { fis -> fis.copyTo(zos) }
                }
            }
        }

        Toast.makeText(
            context,
            context.getString(R.string.export_logs_success),
            Toast.LENGTH_LONG
        ).show()
    } catch (e: Exception) {
        Log.e(logsExportLogTag, "Failed to export logs:$e")
        Toast.makeText(
            context,
            context.getString(R.string.export_logs_failed, e),
            Toast.LENGTH_LONG
        ).show()
    }

}