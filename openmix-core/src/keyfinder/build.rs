fn main() {
    let kf = format!("{}/src/keyfinder", env!("CARGO_MANIFEST_DIR"));
    println!("cargo:rerun-if-changed={}/shim.cpp", kf);
    println!("cargo:rerun-if-changed={}/shim.h", kf);

    let mut build = cc::Build::new();
    build.cpp(true).warnings(false);
    // c++11 for gcc/clang; the cc crate omits -std for MSVC automatically.
    build.std("c++11");
    build.file(format!("{}/shim.cpp", kf));
    for f in [
        "audiodata.cpp",
        "chromagram.cpp",
        "chromatransform.cpp",
        "chromatransformfactory.cpp",
        "constants.cpp",
        "fftadapter.cpp",
        "keyclassifier.cpp",
        "keyfinder.cpp",
        "lowpassfilter.cpp",
        "lowpassfilterfactory.cpp",
        "spectrumanalyser.cpp",
        "temporalwindowfactory.cpp",
        "toneprofiles.cpp",
        "windowfunctions.cpp",
        "workspace.cpp",
    ] {
        build.file(format!("{}/vendor/libkeyfinder/src/{}", kf, f));
    }
    build.include(format!("{}/vendor/libkeyfinder/src", kf));
    // compile() emits cargo:rustc-link-search and cargo:rustc-link-lib itself.
    build.compile("openmix_kf");
}
