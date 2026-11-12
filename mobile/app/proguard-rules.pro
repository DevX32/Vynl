# The catalogue bridge is called from the rust side through jni, so the
# classes and their names have to survive the shrinker.
-keep class com.vynl.vynl_mobile.CatalogueBridge { *; }
-keepclasseswithmembernames class com.vynl.vynl_mobile.CatalogueBridge {
    native <methods>;
}
