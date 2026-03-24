cmake_minimum_required(VERSION 3.16)
project(libre-mp LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Find libturbojpeg
find_library(TURBOJPEG_LIB turbojpeg REQUIRED)

add_executable(libre-mp
    main.cpp
    config.cpp
    payloads.cpp
    client.cpp
    wifi.cpp
    video_streamer.cpp
)

target_link_libraries(libre-mp PRIVATE ${TURBOJPEG_LIB})
