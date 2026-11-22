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
        versionName = "0.1"
        ndk {
            // JNA also ships ABIs for which this build has no Rust library.
            abiFilters += providers.environmentVariable("ABI").orElse("arm64-v8a").get()
        }
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
// reuse existing bindings and JNI libraries while iterating on Kotlin only.
// The script keeps NDK/cargo-ndk discovery out of the Gradle configuration
// cache, which cannot serialize build-script closures.
val repoRoot: File = rootProject.projectDir.parentFile
val skipRust = providers.gradleProperty("skipRust").orNull?.toBoolean() == true

abstract class BuildOsmoRust : Exec() {
    @get:OutputDirectory
    abstract val bindingsDirectory: DirectoryProperty
}

val buildRust = tasks.register<BuildOsmoRust>("buildRust") {
    group = "build"
    description = "Builds the Rust library and generates Kotlin bindings"
    enabled = !skipRust
    workingDir = repoRoot
    bindingsDirectory.set(layout.buildDirectory.dir("generated/osmo-bindings"))
    // Cargo owns the native library's incremental build.
    outputs.upToDateWhen { false }
    commandLine(
        "bash",
        File(repoRoot, "scripts/build-android.sh").absolutePath,
        bindingsDirectory.get().asFile.absolutePath,
    )
}

tasks.named("preBuild").configure { dependsOn(buildRust) }
androidComponents.onVariants { variant ->
    variant.sources.kotlin?.addGeneratedSourceDirectory(buildRust, BuildOsmoRust::bindingsDirectory)
}

// `preBuild` is not an input of AGP's native merge tasks, so a parallel build
// can package jniLibs before cargo-ndk finishes. Make the dependency explicit;
// otherwise an APK can silently contain an outdated native library.
tasks.matching { it.name.startsWith("merge") && it.name.endsWith("NativeLibs") }
    .configureEach { dependsOn(buildRust) }

dependencies {
    implementation(project(":btleplug-jni"))
    implementation(libs.jna) { artifact { type = "aar" } }
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
