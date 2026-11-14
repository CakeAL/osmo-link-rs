plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.cakeal.osmolink"
    compileSdk {
        version = release(37)
    }

    defaultConfig {
        applicationId = "com.cakeal.osmolink"
        minSdk = 29
        targetSdk = 37
        versionCode = 1
        versionName = "1.0"
    }

    buildTypes {
        release {
            optimization {
                enable = true
                packageScope = setOf("androidx.**", "kotlin.**", "kotlinx.**")
            }
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
}

// The BLE stack is Rust. Gradle rebuilds the shared library before packaging,
// so `assembleDebug` alone produces a complete APK. Use `-PskipRust=true` to
// reuse an existing jniLibs output while iterating on Kotlin only.
// The script keeps NDK/cargo-ndk discovery out of the Gradle configuration
// cache, which cannot serialize build-script closures.
val repoRoot: File = rootProject.projectDir.parentFile
val skipRust = providers.gradleProperty("skipRust").orNull?.toBoolean() == true

val buildRust = tasks.register<Exec>("buildRust") {
    group = "build"
    description = "Builds libosmo_bridge.so for Android with cargo-ndk"
    enabled = !skipRust
    workingDir = repoRoot
    commandLine("bash", File(repoRoot, "scripts/build-android.sh").absolutePath)
}

tasks.named("preBuild").configure { dependsOn(buildRust) }

// `preBuild` is not an input of AGP's native merge tasks, so a parallel build
// can package jniLibs before cargo-ndk finishes. Make the dependency explicit;
// otherwise an APK can silently contain the previous libosmo_bridge.so.
tasks.matching { it.name.startsWith("merge") && it.name.endsWith("NativeLibs") }
    .configureEach { dependsOn(buildRust) }

dependencies {
    implementation(project(":btleplug-jni"))
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.graphics)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.decompose.compose)
    implementation(libs.decompose.core)
    testImplementation(libs.junit)
    debugImplementation(libs.androidx.compose.ui.tooling)
}
