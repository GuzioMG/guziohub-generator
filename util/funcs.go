package util

import (
	"os"
	"path/filepath"
)

func DeepOpen(name string, flag int, perm os.FileMode) (*os.File, error) {
	if err := os.MkdirAll(filepath.Dir(name), perm); err != nil {
		return nil, err
	}
	return os.OpenFile(name, flag, perm)
}