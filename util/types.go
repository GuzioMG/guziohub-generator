package util

type TemplateData struct {
	Name    string
	Content *string
}

type LineData struct {
	TypedLength              int
	BytesLength              int
	ProcessedContent         string
	RealNum                  int
	WordCount                int
	TypedLengthWithoutSpaces int
}

type DocumentData struct {
	Lang        string
	Canonical   string
	Title       string
	Header      string
	Description string
}