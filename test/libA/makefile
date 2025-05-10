ifndef libA_included
libA_included:=1

ROOT?=..
include $(ROOT)/../make-fw.mk
include $(ROOT)/libA/makefile_defs.mk

libA_TARGET   := $(OUT)/lib/libA.a
libA_SRCS     := libA.c
libA_OBJS     := $(call mfwOBJECTS,libA,$(libA_SRCS))
libA_LDFLAGS := -lm

ifndef all.defined
all: $(libA_TARGET)
endif

# create target rule dependent on object files, and using specific CFLAGS
$(libA_TARGET): $(libA_OBJS)

# include generated dependency files for the needed source files
-include $(call mfwDEPS,libA,$(libA_SRCS))

# include sub folders
include $(ROOT)/libA/subFolder/makefile
$(libA_TARGET): $(libA_subFolder_OBJS)

endif
