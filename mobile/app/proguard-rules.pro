

-keep class com.vynl.vynl_mobile.CatalogueBridge { *; }
-keepclasseswithmembernames class com.vynl.vynl_mobile.CatalogueBridge {
    native <methods>;
}

-keep class com.google.mlkit.** { *; }
-keep class com.google.android.gms.internal.mlkit_*.** { *; }
-keep class com.google.android.libraries.barhopper.** { *; }
-keep class com.google.barhopper.** { *; }
-keep class com.google.photos.vision.barhopper.** { *; }
-keepattributes *Annotation*, Exceptions, InnerClasses, Signature
