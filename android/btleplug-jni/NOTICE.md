# btleplug Android JNI support classes

`src/main/java` is copied verbatim from btleplug 0.13.5
(`src/droidplug/java/src/main/java`), because btleplug publishes no AAR and its
Android backend loads these classes only through JNI.

Upstream: https://github.com/deviceplug/btleplug

Packages:

- `com.nonpolynomial.btleplug.android.impl` — `Adapter`, `Peripheral`,
  `ScanFilter` and exceptions registered as native methods by
  `btleplug::platform::init`.
- `io.github.gedgygedgy.rust.*` — vendored JNI utility classes (futures, tasks,
  streams, function adapters) used by the backend.

Licensed under MIT / Apache-2.0 / BSD-3-Clause, per the btleplug crate. Keep
these files unmodified; upgrade them together with the `btleplug` crate version.

R8: keep both packages, because they are referenced only from native code.
