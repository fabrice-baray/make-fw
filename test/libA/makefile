ifndef libA.included
libA.included:=1

ROOT?=..
include $(ROOT)/../scripts/make-fw.mk


libA.TARGET   := $(OUT)/lib/libA.a
libA.SRCS     := libA.c
libA.OBJS     := $(patsubst %.c,$(OUT)/obj/libA/%.o,$(libA.SRCS))
libA.DEPS     := $(patsubst %.c,$(OUT)/dep/libA/%.d,$(libA.SRCS))
libA.EXT_DEPS := -lm

ifndef all.defined
all: $(libA.TARGET)
endif

$(libA.TARGET): $(libA.OBJS)
$(libA.OBJS): CFLAGS+=-DLIBA
-include $(libA.DEPS)

endif
