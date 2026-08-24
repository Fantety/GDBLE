# GDBLE and btleplug classes are referenced through plugin metadata and JNI.
-keep class org.gdble.android.GDBLEPlugin { *; }
-keep class com.nonpolynomial.btleplug.android.impl.** { *; }

# Keep native methods
-keepclasseswithmembernames class * {
    native <methods>;
}
