package util

type TemplateData struct {
	Name    string
	Content *string
}

type LineData struct {
	TypedLength      int
	BytesLength      int
	ProcessedContent string
	IsFirst          bool
	WordCount        int
}

type DocumentData struct {
	Lang        string
	Canonical   string
	Title       string
	Header      string
	Description string
}