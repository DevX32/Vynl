import java.io.File
import java.util.Properties

plugins {
    id("com.android.application")
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "com.vynl.vynl_mobile"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        isCoreLibraryDesugaringEnabled = true
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        applicationId = "com.vynl.vynl_mobile"
        minSdk = 24
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
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
        release {
            signingConfig = if (rootProject.file("key.properties").exists()) {
                signingConfigs.getByName("release")
            } else {
                signingConfigs.getByName("debug")
            }
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }
}

dependencies {
    coreLibraryDesugaring("com.android.tools:desugar_jdk_libs:2.1.4")
}

// The catalogue client is Rust, shared with the desktop. It is built with
// cargo-ndk and copied into the jniLibs folder the apk packs.
val catalogueTargets = mapOf(
    "arm64-v8a" to "aarch64-linux-android",
    "armeabi-v7a" to "armv7-linux-androideabi",
    "x86_64" to "x86_64-linux-android",
)

val catalogueRoot = file("../../../crates/vynl-catalogue-ffi")
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
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "-t", "armeabi-v7a",
        "-t", "x86_64",
        "build", "--release",
    )
    inputs.dir(file("../../../crates/vynl-catalogue/src"))
    inputs.dir(file("../../../crates/vynl-catalogue-ffi/src"))
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

flutter {
    source = "../.."
}
