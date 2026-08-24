cmake_minimum_required(VERSION 3.16)
project(silvapi_c C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

add_definitions(-D_GNU_SOURCE)

include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/vendor
)

link_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/lib
    /lib64
    /usr/lib64
)

set(SOURCES
    vendor/cJSON.c
    src/arena.c
    src/spring.c
    src/fredbuf.c
    src/panel_layout.c
    src/theme.c
    src/model.c
    src/gl_backend.c
    src/font.c
    src/json_utils.c
    src/storage.c
    src/http_client.c
    src/import_export.c
    src/ui.c
    src/views/top_bar.c
    src/views/sidebar.c
    src/views/request_view.c
    src/views/response_view.c
    src/views/dialogs.c
    src/main.c
)

add_executable(silvapi-c ${SOURCES})

target_link_libraries(silvapi-c
    glfw
    GL
    curl
    sqlite3
    m
    pthread
)
