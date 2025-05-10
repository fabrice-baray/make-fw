ifndef libB.included
libB.included:=1

ROOT?=..
include $(ROOT)/../scripts/make-fw.mk


libB.TARGET := $(OUT)/lib/libB.a
libB.SRCS   := libB.c
libB.OBJS   := $(patsubst %.c,$(OUT)/obj/libB/%.o,$(libB.SRCS))
libB.DEPS   := $(patsubst %.c,$(OUT)/dep/libB/%.d,$(libB.SRCS))

ifndef all.defined
all: $(libB.TARGET)
endif

$(libB.TARGET): $(libB.OBJS)
$(libB.OBJS): CFLAGS+=-DLIBB
-include $(libB.DEPS)

endif
