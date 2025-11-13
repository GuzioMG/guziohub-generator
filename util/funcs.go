package util

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

func DeepOpen(name string, flag int, perm os.FileMode) (*os.File, error) {
	if err := os.MkdirAll(filepath.Dir(name), perm); err != nil {
		return nil, err
	}
	return os.OpenFile(name, flag, perm)
}

/* path is only for error messages, to help the user identify where the metadata extraction was attempted - so it can be invalid (eg. something like "<internal>" or "<unknown>") if extracting from an "anonymous" source (e.g., a string) */
func ExtractMetadata(line string, path string) (*DocumentData, error) {
	regex, err := regexp.Compile(`(.*?)<html flavor="ghtml" lang="([a-z]{2})" canonical="(.+)" title="(.+)" header="(.+)" description="(.+)">(.*?)`)
	if err != nil {
		return nil, err
	}

	if match := regex.FindStringSubmatch(line); len(match) != 8 {
		return nil, errors.New("file \"" + path + "\" does not appear to be a valid HTML file, of G-HTML flavour (missing, mis(s)-attributed, or invalid opening <html> tag - found \"" + line + "\" instead)")
	} else {
		var err error = nil
		if match[1] != "" || match[7] != "" {
			err = errors.New("file \"" + path + "\" does not appear to be a valid HTML file, of G-HTML flavour (unexpected content before or after opening <html> tag: \"" + line + "\")")
		}
		return &DocumentData{
			Lang:        match[2],
			Canonical:   match[3],
			Title:       match[4],
			Header:      match[5],
			Description: match[6],
		}, err
	}
}

/* path is only for error messages, to help the user identify where the metadata extraction was attempted - so it can be invalid (eg. something like "<internal>" or "<unknown>") if extracting from an "anonymous" source (e.g., a string) */
func ProcessLine(line string, indentation string, path string, lineNum int) (*LineData, error) {

	//Compiling RegEx
	tagFinder, err := regexp.Compile(`<(\S+?) (\S+?)="(.*?)">|</(\S+?)>`)
	if err != nil {
		return nil, err
	}
	envFinder, err := regexp.Compile(`%(\S+?)%`)
	if err != nil {
		return nil, err
	}
	escFinder, err := regexp.Compile(`&(\S+?);`)
	if err != nil {
		return nil, err
	}

	//Counting typed characters and parsing env-vars
	returnedLine := line
	for _, v := range envFinder.FindAllStringSubmatch(line, -1) {
		if val, exists := os.LookupEnv(v[1]); !exists {
			return nil, errors.New("environment variable \"" + v[1] + "\" not set (while processing " + path + " at line #" + fmt.Sprint(lineNum) + " - contents: \"" + line + "\")")
		} else {
			returnedLine = strings.ReplaceAll(returnedLine, v[0], val)
		}
	}
	typedLine := escFinder.ReplaceAllString(tagFinder.ReplaceAllString(returnedLine, ""), "#")
	typedCharCount := strings.Count(typedLine, "") - 1

	//Processing content
	isFirst := false
	prefix := "\n" + indentation + "<br><p class=\"termtxt-default\">&nbsp;$&nbsp;</p><p class=\"termtxt-default typing-animator\">"
	suffix := "</p><p class=\"termtxt-default typing-animator\">_</p>"
	if lineNum == 3 {
		prefix = strings.ReplaceAll(prefix, "<br>", "")
		isFirst = true
	} else {
		prefix = strings.ReplaceAll(prefix, "&nbsp;$", "")
	}
	returnedLine = prefix + returnedLine + suffix

	//Readability guard
	var errNonFatal error = nil
	if typedCharCount > 54 {
		errNonFatal = errors.New("line of length " + fmt.Sprint(typedCharCount) + " is too long to be readable on mobile - over 54 typed characters (while processing " + path + " at line #" + fmt.Sprint(lineNum) + " - contents: \"" + line + "\")")
	}

	return &LineData{
		TypedLength:      typedCharCount,
		BytesLength:      len([]byte(returnedLine)),
		ProcessedContent: returnedLine,
		IsFirst:          isFirst,
		WordCount:        len(strings.Split(typedLine, " ")),
	}, errNonFatal
}
