package strings

import "testing"

func TestReverse(t *testing.T) {
	// This test will fail due to the bug in Reverse
	if Reverse("hello") != "olleh" {
		t.Errorf("Reverse(hello) = %q, want %q", Reverse("hello"), "olleh")
	}
}

func TestUpper(t *testing.T) {
	if Upper("hello") != "HELLO" {
		t.Errorf("Upper(hello) = %q, want %q", Upper("hello"), "HELLO")
	}
}
