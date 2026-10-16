package org.gdble.android

import org.godotengine.godot.Godot
import org.godotengine.godot.plugin.GodotPlugin
import org.godotengine.godot.plugin.UsedByGodot
import android.os.Build
import android.app.Activity
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.view.View

class GDBLEPlugin(godot: Godot) : GodotPlugin(godot) {
    private var receiverActivity: Activity? = null
    @Volatile private var bluetoothContext: Context? = null
    private val powerReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == BluetoothAdapter.ACTION_STATE_CHANGED) {
                // Read physical state rather than trusting broadcast extras.
                reportAdapterState(queryAdapterState())
            }
        }
    }
    companion object {
        init {
            System.loadLibrary("gdble")
        }
    }

    init {
        initializeNative()
    }

    override fun getPluginName(): String = "GDBLE"

    @UsedByGodot
    fun getAndroidApiLevel(): Int = Build.VERSION.SDK_INT

    override fun onMainCreate(activity: Activity?): View? {
        if (activity != null) {
            bluetoothContext = activity.applicationContext
            val filter = IntentFilter(BluetoothAdapter.ACTION_STATE_CHANGED)
            if (Build.VERSION.SDK_INT >= 33) {
                // Bluetooth system broadcasts can originate outside system UID.
                activity.registerReceiver(powerReceiver, filter, Context.RECEIVER_EXPORTED)
            } else {
                @Suppress("DEPRECATION")
                activity.registerReceiver(powerReceiver, filter)
            }
            receiverActivity = activity
        }
        return null
    }

    override fun onMainResume() {
        reportAdapterState(queryAdapterState())
    }

    override fun onMainDestroy() {
        receiverActivity?.unregisterReceiver(powerReceiver)
        receiverActivity = null
        bluetoothContext = null
        reportAdapterState(-1) // Unavailable and release this instance's JNI reference.
    }

    // Invoked via JNI on the BLE worker; holds no Godot object or UI state.
    @Suppress("unused")
    private fun queryAdapterState(): Int {
        val context = bluetoothContext ?: return 0
        return try {
            val manager = context.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager
            manager?.adapter?.state ?: 0
        } catch (_: SecurityException) {
            0 // Missing/revoked permission is unavailable, never optimistic ON.
        }
    }

    private external fun initializeNative(): Boolean
    private external fun reportAdapterState(state: Int)
}
