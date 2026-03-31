# Install script for directory: /home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo

# Set the install prefix
if(NOT DEFINED CMAKE_INSTALL_PREFIX)
  set(CMAKE_INSTALL_PREFIX "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out")
endif()
string(REGEX REPLACE "/$" "" CMAKE_INSTALL_PREFIX "${CMAKE_INSTALL_PREFIX}")

# Set the install configuration name.
if(NOT DEFINED CMAKE_INSTALL_CONFIG_NAME)
  if(BUILD_TYPE)
    string(REGEX REPLACE "^[^A-Za-z0-9_]+" ""
           CMAKE_INSTALL_CONFIG_NAME "${BUILD_TYPE}")
  else()
    set(CMAKE_INSTALL_CONFIG_NAME "Debug")
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

# Install shared libraries without execute permission?
if(NOT DEFINED CMAKE_INSTALL_SO_NO_EXE)
  set(CMAKE_INSTALL_SO_NO_EXE "0")
endif()

# Is this installation the result of a crosscompile?
if(NOT DEFINED CMAKE_CROSSCOMPILING)
  set(CMAKE_CROSSCOMPILING "FALSE")
endif()

# Set path to fallback-tool for dependency-resolution.
if(NOT DEFINED CMAKE_OBJDUMP)
  set(CMAKE_OBJDUMP "/usr/bin/objdump")
endif()

if(NOT CMAKE_INSTALL_LOCAL_ONLY)
  # Include the install script for the subdirectory.
  include("/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/simd/cmake_install.cmake")
endif()

if(NOT CMAKE_INSTALL_LOCAL_ONLY)
  # Include the install script for the subdirectory.
  include("/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/src/md5/cmake_install.cmake")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "lib" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/libturbojpeg.a")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "bin" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE PROGRAM RENAME "tjbench" FILES "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/tjbench-static")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "include" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/include" TYPE FILE FILES "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/turbojpeg.h")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "lib" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib" TYPE STATIC_LIBRARY FILES "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/libjpeg.a")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "bin" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE PROGRAM RENAME "cjpeg" FILES "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/cjpeg-static")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "bin" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE PROGRAM RENAME "djpeg" FILES "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/djpeg-static")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "bin" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE PROGRAM RENAME "jpegtran" FILES "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/jpegtran-static")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "bin" OR NOT CMAKE_INSTALL_COMPONENT)
  if(EXISTS "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/rdjpgcom" AND
     NOT IS_SYMLINK "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/rdjpgcom")
    file(RPATH_CHECK
         FILE "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/rdjpgcom"
         RPATH "")
  endif()
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE EXECUTABLE FILES "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/rdjpgcom")
  if(EXISTS "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/rdjpgcom" AND
     NOT IS_SYMLINK "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/rdjpgcom")
    if(CMAKE_INSTALL_DO_STRIP)
      execute_process(COMMAND "/usr/bin/strip" "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/rdjpgcom")
    endif()
  endif()
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "bin" OR NOT CMAKE_INSTALL_COMPONENT)
  if(EXISTS "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/wrjpgcom" AND
     NOT IS_SYMLINK "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/wrjpgcom")
    file(RPATH_CHECK
         FILE "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/wrjpgcom"
         RPATH "")
  endif()
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/bin" TYPE EXECUTABLE FILES "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/wrjpgcom")
  if(EXISTS "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/wrjpgcom" AND
     NOT IS_SYMLINK "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/wrjpgcom")
    if(CMAKE_INSTALL_DO_STRIP)
      execute_process(COMMAND "/usr/bin/strip" "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/bin/wrjpgcom")
    endif()
  endif()
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "doc" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/share/doc/libjpeg-turbo" TYPE FILE FILES
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/README.ijg"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/README.md"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/example.c"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/tjcomp.c"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/tjdecomp.c"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/tjtran.c"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/doc/libjpeg.txt"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/doc/structure.txt"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/doc/usage.txt"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/doc/wizard.txt"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/LICENSE.md"
    )
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "man" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/share/man/man1" TYPE FILE FILES
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/doc/cjpeg.1"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/doc/djpeg.1"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/doc/jpegtran.1"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/doc/rdjpgcom.1"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/doc/wrjpgcom.1"
    )
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "lib" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/pkgconfig" TYPE FILE FILES "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/pkgscripts/libjpeg.pc")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "lib" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/pkgconfig" TYPE FILE FILES "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/pkgscripts/libturbojpeg.pc")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "lib" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo" TYPE FILE FILES
    "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/pkgscripts/libjpeg-turboConfig.cmake"
    "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/pkgscripts/libjpeg-turboConfigVersion.cmake"
    )
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "lib" OR NOT CMAKE_INSTALL_COMPONENT)
  if(EXISTS "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo/libjpeg-turboTargets.cmake")
    file(DIFFERENT _cmake_export_file_changed FILES
         "$ENV{DESTDIR}${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo/libjpeg-turboTargets.cmake"
         "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/CMakeFiles/Export/f0d506f335508d6549928070f26fb787/libjpeg-turboTargets.cmake")
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
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo" TYPE FILE FILES "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/CMakeFiles/Export/f0d506f335508d6549928070f26fb787/libjpeg-turboTargets.cmake")
  if(CMAKE_INSTALL_CONFIG_NAME MATCHES "^([Dd][Ee][Bb][Uu][Gg])$")
    file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/lib/cmake/libjpeg-turbo" TYPE FILE FILES "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/CMakeFiles/Export/f0d506f335508d6549928070f26fb787/libjpeg-turboTargets-debug.cmake")
  endif()
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "include" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/include" TYPE FILE FILES
    "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/jconfig.h"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/jerror.h"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/jmorecfg.h"
    "/home/Muaz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/turbojpeg-sys-1.1.1/libjpeg-turbo/src/jpeglib.h"
    )
endif()

string(REPLACE ";" "\n" CMAKE_INSTALL_MANIFEST_CONTENT
       "${CMAKE_INSTALL_MANIFEST_FILES}")
if(CMAKE_INSTALL_LOCAL_ONLY)
  file(WRITE "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/install_local_manifest.txt"
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
  file(WRITE "/home/Muaz/Documents/Software/projector/Rust/target/debug/build/turbojpeg-sys-3de4c93511793a4f/out/build/${CMAKE_INSTALL_MANIFEST}"
     "${CMAKE_INSTALL_MANIFEST_CONTENT}")
endif()
