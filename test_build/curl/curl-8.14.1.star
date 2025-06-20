#
# sps2 build recipe for curl
#
# This recipe builds curl from the latest source in its Git repository.
# It enables support for OpenSSL, zlib, and nghttp2 (for HTTP/2).
#

def metadata():
    """Return package metadata."""
    return {
        "name": "curl",
        "version": "8.14.1",
        "description": "A command-line tool and library for transferring data with URL syntax.",
        "license": "CUSTOM",  # MIT-like license, see LICENSES/curl.txt
        "homepage": "https://curl.se",
        "depends": [
            "openssl",
            "zlib",
            "nghttp2",
        ],
        "build_depends": []
    }

def build(ctx):
    """Build the package using the provided context."""
    cleanup(ctx)

    # 2. Fetch the source code from the official Git repository
    fetch(ctx, "https://github.com/curl/curl/releases/download/curl-8_14_1/curl-8.14.1.tar.bz2", "11afb4250beeda00a1c6d00d12374da703d8bc367d292fd4d00da74aa4f84790")

    # 3. Configure the build using CMake.
    # The sps2 `cmake` function handles the complete configure, build,
    # and packaging process. The installation prefix is set automatically.
    cmake(ctx, [
        # Standard release build flags
        "-DCMAKE_BUILD_TYPE=Release",
        "-GNinja",

        # Build shared libraries, which is common for system packages
        "-DBUILD_SHARED_LIBS=ON",

        # Explicitly disable building static libs to save time and space
        "-DBUILD_STATIC_LIBS=OFF",

        # Enable essential features
        "-DCURL_USE_OPENSSL=ON",
        "-DCURL_ZLIB=ON",
        "-DUSE_NGHTTP2=ON",      # For HTTP/2 support
        "-DENABLE_IPV6=ON",      # Enable IPv6 support

        # Disable features not typically needed for a runtime package
        "-DBUILD_TESTING=OFF",
        "-DENABLE_CURL_MANUAL=OFF",
    ])

    # 4. (Optional) Install the package to the system prefix after a
    # successful build.
    # install(ctx)
