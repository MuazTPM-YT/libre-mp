# Install script for directory: C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo

# Set the install prefix
if(NOT DEFINED CMAKE_INSTALL_PREFIX)
  set(CMAKE_INSTALL_PREFIX "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out")
endif()
string(REGEX REPLACE "/$" "" CMAKE_INSTALL_PREFIX "${CMAKE_INSTALL_PREFIX}")

# Set the install configuration name.
if(NOT DEFINED CMAKE_INSTALL_CONFIG_NAME)
  if(BUILD_TYPE)
    string(REGEX REPLACE "^[^A-Za-z0-9_]+" ""
           CMAKE_INSTALL_CONFIG_NAME "${BUILD_TYPE}")
  else()
    set(CMAKE_INSTALL_CONFIG_NAME "Release")
  endif()
  message(STATUS "Install configuration: \"${CMAKE_INSTALL_CONFIG_NAME}\"")
endif()

# Set the component getting installed.
if(NOT CMAKE_INSTALL_COMPONENT)
  if(COMPONENT)
    message(STATUS "Install component: \"${COMPONENT}\"")
    set(CMAKE_INSTALL_COMPONENT "${COMPONENT}")
  else()
    set(CMAKE_INSTALL_COMPONENT)
  endif()
endif()

# Is this installation the result of a crosscompile?
if(NOT DEFINED CMAKE_CROSSCOMPILING)
  set(CMAKE_CROSSCOMPILING "FALSE")
endif()

if(NOT CMAKE_INSTALL_LOCAL_ONLY)
  # Include the install script for the subdirectory.
  include("C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/simd/cmake_install.cmake")
endif()

if(NOT CMAKE_INSTALL_LOCAL_ONLY)
  # Include the install script for the subdirectory.
  include("C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/src/md5/cmake_install.cmake")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "lib" OR NOT CMAKE_INSTALL_COMPONENT)
  if(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Dd][Ee][Bb][Uu][Gg])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/Debug/turbojpeg-static.lib")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ee][Aa][Ss][Ee])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/Release/turbojpeg-static.lib")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Mm][Ii][Nn][Ss][Ii][Zz][Ee][Rr][Ee][Ll])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/MinSizeRel/turbojpeg-static.lib")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ww][Ii][Tt][Hh][Dd][Ee][Bb][Ii][Nn][Ff][Oo])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/RelWithDebInfo/turbojpeg-static.lib")
  endif()
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "bin" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE PROGRAM RENAME "tjbench.exe" FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/${CMAKE_INSTALL_CONFIG_NAME}/tjbench-static.exe")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "include" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/include" TYPE FILE FILES "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/turbojpeg.h")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "lib" OR NOT CMAKE_INSTALL_COMPONENT)
  if(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Dd][Ee][Bb][Uu][Gg])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/Debug/jpeg-static.lib")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ee][Aa][Ss][Ee])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/Release/jpeg-static.lib")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Mm][Ii][Nn][Ss][Ii][Zz][Ee][Rr][Ee][Ll])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/MinSizeRel/jpeg-static.lib")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ww][Ii][Tt][Hh][Dd][Ee][Bb][Ii][Nn][Ff][Oo])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/RelWithDebInfo/jpeg-static.lib")
  endif()
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "bin" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE PROGRAM RENAME "cjpeg.exe" FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/${CMAKE_INSTALL_CONFIG_NAME}/cjpeg-static.exe")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "bin" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE PROGRAM RENAME "djpeg.exe" FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/${CMAKE_INSTALL_CONFIG_NAME}/djpeg-static.exe")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "bin" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE PROGRAM RENAME "jpegtran.exe" FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/${CMAKE_INSTALL_CONFIG_NAME}/jpegtran-static.exe")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "bin" OR NOT CMAKE_INSTALL_COMPONENT)
  if(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Dd][Ee][Bb][Uu][Gg])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE EXECUTABLE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/Debug/rdjpgcom.exe")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ee][Aa][Ss][Ee])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE EXECUTABLE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/Release/rdjpgcom.exe")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Mm][Ii][Nn][Ss][Ii][Zz][Ee][Rr][Ee][Ll])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE EXECUTABLE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/MinSizeRel/rdjpgcom.exe")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ww][Ii][Tt][Hh][Dd][Ee][Bb][Ii][Nn][Ff][Oo])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE EXECUTABLE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/RelWithDebInfo/rdjpgcom.exe")
  endif()
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "bin" OR NOT CMAKE_INSTALL_COMPONENT)
  if(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Dd][Ee][Bb][Uu][Gg])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE EXECUTABLE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/Debug/wrjpgcom.exe")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ee][Aa][Ss][Ee])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE EXECUTABLE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/Release/wrjpgcom.exe")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Mm][Ii][Nn][Ss][Ii][Zz][Ee][Rr][Ee][Ll])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE EXECUTABLE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/MinSizeRel/wrjpgcom.exe")
  elseif(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ww][Ii][Tt][Hh][Dd][Ee][Bb][Ii][Nn][Ff][Oo])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE EXECUTABLE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/RelWithDebInfo/wrjpgcom.exe")
  endif()
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "doc" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/share/doc/libjpeg-turbo" TYPE FILE FILES
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/README.ijg"
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/README.md"
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/example.c"
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/tjcomp.c"
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/tjdecomp.c"
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/tjtran.c"
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/doc/libjpeg.txt"
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/doc/structure.txt"
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/doc/usage.txt"
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/doc/wizard.txt"
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/LICENSE.md"
    )
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "lib" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/pkgconfig" TYPE FILE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/pkgscripts/libjpeg.pc")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "lib" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/pkgconfig" TYPE FILE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/pkgscripts/libturbojpeg.pc")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "lib" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo" TYPE FILE FILES
    "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/pkgscripts/libjpeg-turboConfig.cmake"
    "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/pkgscripts/libjpeg-turboConfigVersion.cmake"
    )
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "lib" OR NOT CMAKE_INSTALL_COMPONENT)
  if(EXISTS "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo/libjpeg-turboTargets.cmake")
    file(DIFFERENT _cmake_export_file_changed FILES
         "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo/libjpeg-turboTargets.cmake"
         "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/CMakeFiles/Export/f0d506f335508d6549928070f26fb787/libjpeg-turboTargets.cmake")
    if(_cmake_export_file_changed)
      file(GLOB _cmake_old_config_files "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo/libjpeg-turboTargets-*.cmake")
      if(_cmake_old_config_files)
        string(REPLACE ";" ", " _cmake_old_config_files_text "${_cmake_old_config_files}")
        message(STATUS "Old export file \"$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo/libjpeg-turboTargets.cmake\" will be replaced.  Removing files [${_cmake_old_config_files_text}].")
        unset(_cmake_old_config_files_text)
        file(REMOVE ${_cmake_old_config_files})
      endif()
      unset(_cmake_old_config_files)
    endif()
    unset(_cmake_export_file_changed)
  endif()
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo" TYPE FILE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/CMakeFiles/Export/f0d506f335508d6549928070f26fb787/libjpeg-turboTargets.cmake")
  if(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Dd][Ee][Bb][Uu][Gg])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo" TYPE FILE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/CMakeFiles/Export/f0d506f335508d6549928070f26fb787/libjpeg-turboTargets-debug.cmake")
  endif()
  if(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Mm][Ii][Nn][Ss][Ii][Zz][Ee][Rr][Ee][Ll])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo" TYPE FILE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/CMakeFiles/Export/f0d506f335508d6549928070f26fb787/libjpeg-turboTargets-minsizerel.cmake")
  endif()
  if(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ww][Ii][Tt][Hh][Dd][Ee][Bb][Ii][Nn][Ff][Oo])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo" TYPE FILE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/CMakeFiles/Export/f0d506f335508d6549928070f26fb787/libjpeg-turboTargets-relwithdebinfo.cmake")
  endif()
  if(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Rr][Ee][Ll][Ee][Aa][Ss][Ee])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo" TYPE FILE FILES "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/CMakeFiles/Export/f0d506f335508d6549928070f26fb787/libjpeg-turboTargets-release.cmake")
  endif()
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "include" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/include" TYPE FILE FILES
    "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/jconfig.h"
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/jerror.h"
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/jmorecfg.h"
    "C:/Users/User/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/jpeglib.h"
    )
endif()

string(REPLACE ";" "\n" CMAKE_INSTALL_MANIFEST_CONTENT
       "${CMAKE_INSTALL_MANIFEST_FILES}")
if(CMAKE_INSTALL_LOCAL_ONLY)
  file(WRITE "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/install_local_manifest.txt"
     "${CMAKE_INSTALL_MANIFEST_CONTENT}")
endif()
if(CMAKE_INSTALL_COMPONENT)
  if(CMAKE_INSTALL_COMPONENT MATCHES "^[a-zA-Z0-9_.+-]+$")
    set(CMAKE_INSTALL_MANIFEST "install_manifest_${CMAKE_INSTALL_COMPONENT}.txt")
  else()
    string(MD5 CMAKE_INST_COMP_HASH "${CMAKE_INSTALL_COMPONENT}")
    set(CMAKE_INSTALL_MANIFEST "install_manifest_${CMAKE_INST_COMP_HASH}.txt")
    unset(CMAKE_INST_COMP_HASH)
  endif()
else()
  set(CMAKE_INSTALL_MANIFEST "install_manifest.txt")
endif()

if(NOT CMAKE_INSTALL_LOCAL_ONLY)
  file(WRITE "C:/Users/User/Documents/libre-mp/Rust/target/release/build/turbojpeg-sys-dab6ee30a1f29961/out/build/${CMAKE_INSTALL_MANIFEST}"
     "${CMAKE_INSTALL_MANIFEST_CONTENT}")
endif()
