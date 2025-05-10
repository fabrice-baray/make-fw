ifndef libA_subFolder_included
libA_subFolder_included:=1

ROOT?=../..
include $(ROOT)/../make-fw.mk

TARGET   := $(OUT)/lib/libA.a
SRCS     := other.c
OBJS     := $(call mfwOBJECTS,libA/subFolder,$(SRCS))

ifndef all.defined
all: $(OBJS)
endif

# create target rule dependent on object files, and using specific CFLAGS
$(TARGET): $(OBJS)

# include generated dependency files for the needed source files
-include $(call mfwDEPS,libA/subFolder,$(SRCS))

endif
