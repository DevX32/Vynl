import java.io.File
import java.util.Properties
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "com.vynl.vynl_mobile"
    compileSdk = 37
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "com.vynl.vynl_mobile"
        minSdk = 24
        targetSdk = 36
        versionCode = 2027
        versionName = "0.4.5"
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }
    }

    signingConfigs {
        create("release") {
            val keystoreProps = Properties()
            val keystorePropsFile = rootProject.file("key.properties")
            if (keystorePropsFile.exists()) {
                keystorePropsFile.inputStream().use { keystoreProps.load(it) }
                storeFile = File(keystoreProps.getProperty("storeFile"))
                storePassword = keystoreProps.getProperty("storePassword")
                keyAlias = keystoreProps.getProperty("keyAlias")
                keyPassword = keystoreProps.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        debug {
            // The flutter build keeps the real id, so the debug app lives
            // beside it until the compose one takes over.
            applicationIdSuffix = ".compose"
            isMinifyEnabled = false
        }
        release {
            signingConfig = if (rootProject.file("key.properties").exists()) {
                signingConfigs.getByName("release")
            } else {
                signingConfigs.getByName("debug")
            }
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }

    compileOptions {
        isCoreLibraryDesugaringEnabled = true
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
    }

    packaging {
        jniLibs {
            useLegacyPackaging = false
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
    }
}

dependencies {
    implementation(platform("androidx.compose:compose-bom:2026.09.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.core:core-ktx:1.19.1")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.11.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.11.0")
    implementation("androidx.media3:media3-exoplayer:1.11.1")
    implementation("androidx.media3:media3-session:1.11.1")
    implementation("io.coil-kt.coil3:coil-compose:3.6.3")
    implementation("io.coil-kt.coil3:coil-network-okhttp:3.6.3")
    coreLibraryDesugaring("com.android.tools:desugar_jdk_libs:2.1.5")
}

// The catalogue client is Rust, written inside the app. It is built with
// cargo-ndk and copied into the jniLibs folder the apk packs.
val catalogueTargets = mapOf(
    "arm64-v8a" to "aarch64-linux-android",
    "armeabi-v7a" to "armv7-linux-androideabi",
    "x86_64" to "x86_64-linux-android",
)

val catalogueRoot = file("../rust")
val jniLibsDir = file("src/main/jniLibs")

val buildCatalogueLibrary by tasks.registering(Exec::class) {
    group = "build"
    description = "Builds the shared catalogue library for every android abi"
    workingDir = catalogueRoot
    val ndkHome = System.getenv("ANDROID_NDK_HOME")
        ?: System.getenv("ANDROID_NDK_ROOT")
        ?: "${rootProject.projectDir}/../ndk"
    environment("ANDROID_NDK_HOME", ndkHome)
    commandLine(
        "cargo",
        "ndk",
        "-t",
        "arm64-v8a",
        "-t",
        "armeabi-v7a",
        "-t",
        "x86_64",
        "build",
        "--release",
    )
    inputs.dir(file("../rust/src"))
    outputs.dir(jniLibsDir)
    doLast {
        catalogueTargets.forEach { (abi, target) ->
            val built = catalogueRoot.resolve("target/$target/release/libvynl_catalogue.so")
            if (!built.exists()) {
                throw GradleException("the catalogue library for $abi was not built")
            }
            val into = jniLibsDir.resolve(abi)
            into.mkdirs()
            built.copyTo(into.resolve("libvynl_catalogue.so"), overwrite = true)
        }
    }
}

tasks.matching { it.name.startsWith("merge") && it.name.contains("JniLibFolders") }.configureEach {
    dependsOn(buildCatalogueLibrary)
}

tasks.matching { it.name.startsWith("merge") && it.name.contains("Assets") }.configureEach {
    dependsOn(buildCatalogueLibrary)
}
