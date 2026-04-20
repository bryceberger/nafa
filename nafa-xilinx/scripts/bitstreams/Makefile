# call with something like:
# make $(./parts.sh all-idcodes | sed 's,.*,generated/vivado/\0.bin,')

data := \
	empty.v \
	empty.xdc \
	make.tcl \

vivado := vivado -nolog -nojou -notrace -mode batch -source make.tcl -tclargs

.SECONDARY: generated/readback/ generated/vivado/

generated/%/:
	mkdir -p $@

.SECONDEXPANSION:

generated/vivado/%.bin generated/vivado/%.rbd generated/vivado/%.msd &: ${data} | $$(dir $$@)
	${vivado} -out $(dir $@) -device $(shell ./parts.sh $(basename $(notdir $@))) -name $(basename $(notdir $@))

generated/readback/%.rbd: $$(addprefix generated/vivado/,$$(notdir $$@)) | $$(dir $$@)
	python3 unbin.py $< $@
