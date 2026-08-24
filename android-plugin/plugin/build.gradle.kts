plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

val generatedAssets = layout.buildDirectory.dir("generated/assets")
val generatedJniLibs = layout.buildDirectory.dir("generated/jniLibs")

val prepareGdextensionAsset by tasks.registering(Copy::class) {
    from(rootProject.file("../addons/gdble/gdble.gdextension"))
    into(generatedAssets.map { it.dir("addons/gdble") })
}

android {
    namespace = "org.gdble.android"
    compileSdk = 34

    defaultConfig {
        minSdk = 23
        consumerProguardFiles("consumer-rules.pro")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    sourceSets {
        getByName("main") {
            assets.srcDir(generatedAssets)
            jniLibs.srcDir(generatedJniLibs)
        }
    }
}

tasks.named("preBuild").configure {
    dependsOn(prepareGdextensionAsset)
}

dependencies {
    compileOnly("org.godotengine:godot:4.2.2.stable")
}
