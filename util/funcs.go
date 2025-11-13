package util

import (
	"errors"
	"os"
	"path/filepath"
)

func DeepOpen(name string, flag int, perm os.FileMode) (*os.File, error) {
	if err := os.MkdirAll(filepath.Dir(name), perm); err != nil {
		return nil, err
	}
	return os.OpenFile(name, flag, perm)
}

/* path is only for error messages, to help the user identify where the metadata extraction was attempted - so it can be invalid (eg. something like "<internal>" or "<unknown>") if extracting from an "anonymous" source (e.g., a string) */
func ExtractMetadata(line string, path string) (*DocumentData, error) {
	return nil, errors.New("ExtractMetadata not implemented") //TODO: implement
}

/* path and lineNum are only for error messages, to help the user identify where the metadata extraction was attempted - so the path can be invalid (eg. something like "<internal>" or "<unknown>") if processing an "anonymous" source (e.g., a string), while lineNum can be 0 in such case */
func ProcessLine(line string, indentation string, path string, lineNum int) (*LineData, error) {
	return nil, errors.New("ProcessLine not implemented") //TODO: implement
}