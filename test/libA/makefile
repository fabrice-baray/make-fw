ifndef libA_included
libA_included:=1

ROOT?=..
include $(ROOT)/../make-fw.mk


libA_TARGET   := $(OUT)/lib/libA.a
libA_SRCS     := libA.c
libA_OBJS     := $(call mfwOBJECTS,libA,$(libA_SRCS))
libA_DEPS     := $(call mfwDEPS,libA,$(LIBA_SRCS))
libA_LDFLAGS := -lm

ifndef all.defined
all: $(libA_TARGET)
endif

$(libA_TARGET): $(libA_OBJS)
$(libA_OBJS): CFLAGS+=-DLIBA
-include $(libA_DEPS)

endif
