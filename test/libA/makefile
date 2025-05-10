ifndef libA_included
libA_included:=1

ROOT?=..
include $(ROOT)/../make-fw.mk


libA_TARGET   := $(OUT)/lib/libA.a
libA_SRCS     := libA.c
libA_OBJS     := $(call mfwOBJECTS,libA,$(libA_SRCS))
libA_LDFLAGS := -lm

ifndef all.defined
all: $(libA_TARGET)
endif

# create target rule dependent on object files, and using specific CFLAGS
$(libA_TARGET): $(libA_OBJS)
$(libA_OBJS): CFLAGS+=-DLIBA

# include generated dependency files for the needed source files
-include $(call mfwDEPS,libA,$(LIBA_SRCS))

# include sub folders
include $(ROOT)/libA/subFolder/makefile

endif
