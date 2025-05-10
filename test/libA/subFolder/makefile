ifndef libA_subFolder_included
libA_subFolder_included:=1

ROOT?=../..
include $(ROOT)/../make-fw.mk

#include settings of libA
include $(ROOT)/libA/makefile_defs.mk

libA_subFolder_SRCS     := other.c
libA_subFolder_OBJS     := $(call mfwOBJECTS,libA/subFolder,$(libA_subFolder_SRCS))

ifndef all.defined
all: $(libA_subFolder_OBJS)
endif

# include generated dependency files for the needed source files
-include $(call mfwDEPS,libA/subFolder,$(libA_subFolder_SRCS))

endif
