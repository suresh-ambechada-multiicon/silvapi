# Silvapi C Frontend
#
# Layered build: each libs/<name> compiles to build/lib<name>.a and is given ONLY
# the include paths of the layers it is allowed to depend on. A layering violation
# (e.g. net/ reaching for a GUI header) fails at compile time, not review time.

CC      ?= gcc
AR      ?= ar
BUILD   := build
BIN     := bin

PKGS      := freetype2 harfbuzz fontconfig
PKG_CFLAGS := $(shell pkg-config --cflags $(PKGS) 2>/dev/null)
PKG_LIBS   := $(shell pkg-config --libs   $(PKGS) 2>/dev/null)

WARN     := -Wall -Wextra
BASE_CFLAGS := -std=c11 $(WARN) -O2 -g -D_GNU_SOURCE -MMD -MP
VENDOR_INC  := -I vendor

# ---------------------------------------------------------------------------
# Layer include paths. A layer sees itself plus everything strictly below it.
# ---------------------------------------------------------------------------

INC_base  := -I libs/base/include
INC_ds    := $(INC_base) -I libs/ds/include
INC_gfx   := $(INC_ds)   -I libs/gfx/include
INC_text  := $(INC_gfx)  -I libs/text/include
INC_icons := $(INC_gfx)  -I libs/icons/include
INC_json  := $(INC_base) -I libs/json/include
INC_net   := $(INC_ds)   -I libs/net/include
INC_input := $(INC_base) -I libs/input/include
INC_ui    := $(INC_text) $(INC_icons) $(INC_json) $(INC_input) -I libs/ui/include
INC_app   := $(INC_ui) $(INC_net) $(INC_json) $(INC_input) -I app/include

# net/ must stay GUI-free and headless-linkable; it deliberately gets no gfx/ui paths.

LIBS := base ds gfx text icons json net input ui

base_SRCS  := $(wildcard libs/base/src/*.c)
base_SRCS  := $(filter-out libs/base/src/test_%.c,$(base_SRCS))
ds_SRCS    := $(wildcard libs/ds/src/*.c)
gfx_SRCS   := $(wildcard libs/gfx/src/*.c)
text_SRCS  := $(wildcard libs/text/src/*.c)
icons_SRCS := $(wildcard libs/icons/src/*.c)
json_SRCS  := $(wildcard libs/json/src/*.c)
net_SRCS   := $(wildcard libs/net/src/*.c)
input_SRCS := $(wildcard libs/input/src/*.c)
ui_SRCS    := $(filter-out %/test_style.c %/test_fuzzy.c,$(wildcard libs/ui/src/*.c)) \
              $(wildcard libs/ui/components/*.c)

APP_SRCS    := $(wildcard app/src/*.c) $(wildcard app/src/views/*.c)
VENDOR_SRCS := vendor/cJSON.c

TARGET := $(BIN)/silvapi-c

LDFLAGS := -lglfw -lGL -lcurl -lsqlite3 -lm -lpthread $(PKG_LIBS)

# app and vendor need every public include path
APP_CFLAGS    := $(BASE_CFLAGS) $(INC_app) $(VENDOR_INC) $(PKG_CFLAGS)
VENDOR_CFLAGS := -std=c11 -O2 -g -D_GNU_SOURCE -MMD -MP $(VENDOR_INC)

obj = $(BUILD)/$(1)/$(patsubst %.c,%.o,$(notdir $(2)))

.PHONY: all clean run test test-style test-fuzzy dirs
all: $(TARGET)

# --- per-library rules, generated ------------------------------------------
define LIB_RULE
$(1)_OBJS := $$(patsubst %.c,$(BUILD)/%.o,$$($(1)_SRCS))
$(BUILD)/libs/$(1)/src/%.o: libs/$(1)/src/%.c
	@mkdir -p $$(dir $$@)
	$(CC) $(BASE_CFLAGS) $$(INC_$(1)) $(VENDOR_INC) $(PKG_CFLAGS) -c $$< -o $$@
# A library may keep one component per file under components/; same include set.
$(BUILD)/libs/$(1)/components/%.o: libs/$(1)/components/%.c
	@mkdir -p $$(dir $$@)
	$(CC) $(BASE_CFLAGS) $$(INC_$(1)) $(VENDOR_INC) $(PKG_CFLAGS) -c $$< -o $$@
$(BUILD)/lib$(1).a: $$($(1)_OBJS)
	@mkdir -p $$(dir $$@)
	$(AR) rcs $$@ $$^
ALL_LIBS += $(BUILD)/lib$(1).a
ALL_OBJS += $$($(1)_OBJS)
endef
$(foreach L,$(LIBS),$(eval $(call LIB_RULE,$(L))))

APP_OBJS    := $(patsubst %.c,$(BUILD)/%.o,$(APP_SRCS))
VENDOR_OBJS := $(patsubst %.c,$(BUILD)/%.o,$(VENDOR_SRCS))

$(BUILD)/app/%.o: app/%.c
	@mkdir -p $(dir $@)
	$(CC) $(APP_CFLAGS) -c $< -o $@

$(BUILD)/vendor/%.o: vendor/%.c
	@mkdir -p $(dir $@)
	$(CC) $(VENDOR_CFLAGS) -c $< -o $@

# The archives are still mutually referential while the layering is finished,
# so they link as a group rather than relying on a strict topological order.
$(TARGET): $(APP_OBJS) $(VENDOR_OBJS) $(ALL_LIBS)
	@mkdir -p $(BIN)
	$(CC) $(APP_OBJS) $(VENDOR_OBJS) \
	  -Wl,--start-group $(ALL_LIBS) -Wl,--end-group \
	  -o $@ $(LDFLAGS)

# Unit tests that do not need a GL context.
test-style: libs/ui/src/test_style.c libs/ui/src/style.c
	@mkdir -p $(BIN)
	$(CC) $(BASE_CFLAGS) $(INC_ui) $(VENDOR_INC) $(PKG_CFLAGS) \
	  libs/ui/src/test_style.c libs/ui/src/style.c -o $(BIN)/test_style -lm
	./$(BIN)/test_style

test-fuzzy: libs/ui/src/test_fuzzy.c libs/ui/src/fuzzy.c
	@mkdir -p $(BIN)
	$(CC) $(BASE_CFLAGS) $(INC_ui) $(VENDOR_INC) $(PKG_CFLAGS) \
	  libs/ui/src/test_fuzzy.c libs/ui/src/fuzzy.c -o $(BIN)/test_fuzzy -lm
	./$(BIN)/test_fuzzy

test: test-style test-fuzzy $(BUILD)/libbase.a $(BUILD)/libui.a
	@mkdir -p $(BIN)
	$(CC) $(BASE_CFLAGS) $(INC_ui) libs/base/src/test_arena_spring.c \
	  $(BUILD)/libui.a $(BUILD)/libbase.a -o $(BIN)/test_suite -lm
	./$(BIN)/test_suite

run: $(TARGET)
	./$(TARGET)

clean:
	rm -rf $(BUILD) $(TARGET) $(BIN)/test_suite

-include $(shell find $(BUILD) -name '*.d' 2>/dev/null)
