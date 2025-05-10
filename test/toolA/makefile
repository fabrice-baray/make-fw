ifndef toolA.included
toolA.included:=1

ROOT?=..
include $(ROOT)/../scripts/make-fw.mk


toolA.TARGET := $(OUT)/bin/toolA
toolA.SRCS   := test.c
toolA.OBJS   := $(patsubst %.c,$(OUT)/obj/toolA/%.o,$(toolA.SRCS))
toolA.DEPS   := $(patsubst %.c,$(OUT)/dep/toolA/%.d,$(toolA.SRCS))

ifndef all.defined
all: $(toolA.OBJS)
endif

.PHONY: toolA
toolA:$(toolA.TARGET)
$(toolA.TARGET): $(toolA.OBJS)
-include $(toolA.DEPS)

#(eval $(call add_ar_dep,$(toolA.TARGET),libA))
include $(ROOT)/libA/makefile
$(toolA.TARGET): LDFLAGS+=$(libA.LDFLAGS)
$(toolA.TARGET): $(libA.TARGET)

include $(ROOT)/libB/makefile
$(toolA.TARGET): $(libB.TARGET)
$(toolA.TARGET): LDFLAGS+=$(libB.LDFLAGS)
$(toolA.TARGET): LDLIBS+=-lB

endif
