ifndef toolA.included
toolA.included:=1

ROOT?=..
include $(ROOT)/../scripts/make-fw.mk


toolA_TARGET := $(OUT)/bin/toolA
toolA_SRCS   := test.c
toolA_OBJS   := $(call mfwOBJECTS,toolA,$(toolA_SRCS))
toolA_DEPS   := $(call mfwDEPS,toolA,$(toolA_SRCS))

ifndef all.defined
all: $(toolA_OBJS)
endif

.PHONY: toolA
toolA:$(toolA_TARGET)
$(toolA_TARGET): $(toolA_OBJS)
-include $(toolA_DEPS)

# dependent on libA
include $(ROOT)/libA/makefile
$(toolA_TARGET): LDFLAGS+=$(libA_LDFLAGS)
$(toolA_TARGET): $(libA_TARGET)

# dependent on libB
include $(ROOT)/libB/makefile
$(toolA_TARGET): $(libB_TARGET)
$(toolA_TARGET): LDFLAGS+=$(libB_LDFLAGS)
$(toolA_TARGET): LDLIBS+=-lB

endif
