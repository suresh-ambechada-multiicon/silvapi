CC = gcc
CFLAGS = -std=c11 -Wall -Wextra -O2 -D_GNU_SOURCE -I./include -I./vendor
LDFLAGS = -L./lib -L/lib64 -L/usr/lib64 -lglfw -lGL -lcurl -lsqlite3 -lm -lpthread

SRCS = \
	vendor/cJSON.c \
	src/fredbuf.c \
	src/panel_layout.c \
	src/theme.c \
	src/model.c \
	src/gl_backend.c \
	src/font.c \
	src/json_utils.c \
	src/storage.c \
	src/http_client.c \
	src/import_export.c \
	src/ui.c \
	src/views/top_bar.c \
	src/views/sidebar.c \
	src/views/request_view.c \
	src/views/response_view.c \
	src/views/dialogs.c \
	src/main.c

OBJS = $(SRCS:.c=.o)
TARGET = bin/silvapi-c

all: $(TARGET)

bin:
	mkdir -p bin

$(TARGET): bin $(OBJS)
	$(CC) $(OBJS) -o $@ $(LDFLAGS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJS) $(TARGET)

run: $(TARGET)
	./$(TARGET)

.PHONY: all clean run
