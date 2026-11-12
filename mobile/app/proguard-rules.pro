# The catalogue bridge is called from the rust side through jni, so the
# classes and their names have to survive the shrinker.
-keep class com.vynl.vynl_mobile.CatalogueBridge { *; }
-keepclasseswithmembernames class com.vynl.vynl_mobile.CatalogueBridge {
    native <methods>;
}

# ML Kit loads its component registrars by class name (from manifest
# meta-data and *.properties files shipped inside the AARs). Nothing in
# the app's code references them, so without these keeps the shrinker
# renames or drops them and the first call into ML Kit - opening the QR
# scanner on the pairing screen - throws ClassNotFoundException and the
# app dies. The bundled model classes are loaded the same way.
-keep class com.google.mlkit.** { *; }
-keep class com.google.android.gms.internal.mlkit_*.** { *; }
-keep class com.google.android.libraries.barhopper.** { *; }
-keep class com.google.barhopper.** { *; }
-keep class com.google.photos.vision.barhopper.** { *; }
-keepattributes *Annotation*, Exceptions, InnerClasses, Signature
