ifndef libB_included
libB_included:=1

ROOT?=..
include $(ROOT)/../make-fw.mk


libB_TARGET  := $(OUT)/lib/libB.so
libB_SRCS    := libB.c
libB_OBJS    := $(call mfwOBJECTS,libB,$(libB_SRCS))
libB_LDFLAGS :=

ifndef all.defined
all: $(libB_TARGET)
endif

# create target rule dependent on object files, and using specific CFLAGS
$(libB_TARGET): $(libB_OBJS)
$(libB_OBJS): CFLAGS+=-DLIBB -shared

# include generated dependency files for the needed source files
-include $(call mfwDEPS,libB,$(libB_SRCS))

endif
