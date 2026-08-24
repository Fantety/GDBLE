package org.gdble.android

import org.godotengine.godot.Godot
import org.godotengine.godot.plugin.GodotPlugin

class GDBLEPlugin(godot: Godot) : GodotPlugin(godot) {
    companion object {
        init {
            System.loadLibrary("gdble")
        }
    }

    init {
        initializeNative()
    }

    override fun getPluginName(): String = "GDBLE"

    private external fun initializeNative(): Boolean
}
