ifndef libA.included
libA.included:=1

ROOT?=..
include $(ROOT)/../scripts/make-fw.mk


libA.TARGET   := $(OUT)/lib/libA.a
libA.SRCS     := libA.c
libA.OBJS     := $(call make-fw.objects,libA,$(libA.SRCS))
libA.DEPS     := $(call make-fw.deps,libA,$(LIBA.SRCS))
libA.LDFLAGS := -lm

ifndef all.defined
all: $(libA.TARGET)
endif

$(libA.TARGET): $(libA.OBJS)
$(libA.OBJS): CFLAGS+=-DLIBA
-include $(libA.DEPS)

endif
