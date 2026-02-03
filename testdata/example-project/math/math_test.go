package math

import "testing"

func TestAdd(t *testing.T) {
	if Add(2, 3) != 5 {
		t.Error("Add(2, 3) should equal 5")
	}
}

func TestSubtract(t *testing.T) {
	if Subtract(5, 3) != 2 {
		t.Error("Subtract(5, 3) should equal 2")
	}
}

func TestMultiply(t *testing.T) {
	// This test will fail due to the bug in Multiply
	if Multiply(2, 3) != 6 {
		t.Errorf("Multiply(2, 3) = %d, want 6", Multiply(2, 3))
	}
}
