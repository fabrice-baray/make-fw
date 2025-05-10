ifndef libA_subFolder_included
libA_subFolder_included:=1

ROOT?=../..
include $(ROOT)/../make-fw.mk

libA_subFolder_TARGET   := $(OUT)/lib/libA.a
libA_subFolder_SRCS     := other.c
libA_subFolder_OBJS     := $(call mfwOBJECTS,libA/subFolder,$(libA_subFolder_SRCS))

ifndef all.defined
all: $(libA_subFolder_OBJS)
endif

# create target rule dependent on object files, and using specific CFLAGS
$(libA_subFolder_TARGET): $(libA_subFolder_OBJS)

# include generated dependency files for the needed source files
-include $(call mfwDEPS,libA/subFolder,$(libA_subFolder_SRCS))

endif
