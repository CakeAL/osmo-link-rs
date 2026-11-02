# btleplug Android JNI support classes

`src/main/java` is copied from btleplug 0.13.5
(`src/droidplug/java/src/main/java`), because btleplug publishes no AAR and its
Android backend loads these classes only through JNI.

One local patch in `com.nonpolynomial.btleplug.android.impl.Peripheral`
(`setCharacteristicNotification`): prefer the notify CCCD value when a
characteristic advertises both notify and indicate, use the API 33+
`writeDescriptor(descriptor, value)` overload, and log the properties/value.
Re-apply it when upgrading the vendored classes.

Upstream: https://github.com/deviceplug/btleplug

Packages:

- `com.nonpolynomial.btleplug.android.impl` — `Adapter`, `Peripheral`,
  `ScanFilter` and exceptions registered as native methods by
  `btleplug::platform::init`.
- `io.github.gedgygedgy.rust.*` — vendored JNI utility classes (futures, tasks,
  streams, function adapters) used by the backend.

Licensed under MIT / Apache-2.0 / BSD-3-Clause, per the btleplug crate. Apart
from the patch above, keep these files unmodified and upgrade them together with
the `btleplug` crate version.

R8: keep both packages, because they are referenced only from native code.
