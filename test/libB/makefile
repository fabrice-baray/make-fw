ifndef libB_included
libB_included:=1

ROOT?=..
include $(ROOT)/../scripts/make-fw.mk


libB_TARGET  := $(OUT)/lib/libB.so
libB_SRCS    := libB.c
libB_OBJS    := $(call mfwOBJECTS,libB,$(libB_SRCS))
libB_DEPS    := $(call mfwDEPS,libB,$(libB_SRCS))
libB_LDFLAGS :=

ifndef all.defined
all: $(libB_TARGET)
endif

$(libB_TARGET): $(libB_OBJS)
$(libB_OBJS): CFLAGS+=-DLIBB -shared
-include $(libB_DEPS)

endif
