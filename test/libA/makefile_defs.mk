ifndef libA_defs_included
libA_defs_included:=1

# example of compilation flag used for all objects of libA
libA_CFLAGS:=-DLIBA
$(OUT)/$(mfwOBJ)/libA/%: CFLAGS+=$(libA_CFLAGS)

endif
