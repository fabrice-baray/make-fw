ifndef libB.included
libB.included:=1

ROOT?=..
include $(ROOT)/../scripts/make-fw.mk


libB.TARGET  := $(OUT)/lib/libB.so
libB.SRCS    := libB.c
libB.OBJS    := $(call make-fw.objects,libB,$(libB.SRCS))
libB.DEPS    := $(call make-fw.deps,libB,$(libB.SRCS))
libB.LDFLAGS :=

ifndef all.defined
all: $(libB.TARGET)
endif

$(libB.TARGET): $(libB.OBJS)
$(libB.OBJS): CFLAGS+=-DLIBB -shared
-include $(libB.DEPS)

endif
