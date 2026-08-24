package gnarkbench

import "fmt"

func errReluBitsMissing(task string, layer int) error {
	return fmt.Errorf("%s: layer %d has a ReLU but no measured bit width; refusing to emit a gadget whose range check would be vacuous", task, layer)
}

func errReluCount(task string, emitted, published int) error {
	return fmt.Errorf("%s: emitted %d activations but bench/TASKS.md fixes %d; activations are reported separately from MACs and the count drifted", task, emitted, published)
}

func errPositionOutOfRange(pos SecretPosition, n int) error {
	return fmt.Errorf("position %s is outside the assignment (len %d)", pos, n)
}

func errUnknownKind(spec Spec) error {
	return fmt.Errorf("%s: unknown kind %q", spec.Label, spec.Kind)
}
