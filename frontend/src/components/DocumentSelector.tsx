import {useState} from "react";
import {useDocuments} from "@/hooks/useDocuments";
import {LoadingSpinner, EmptyState} from "@/components/primitives";
import {api} from "@/api";
import type {DocumentListItem, Document} from "@/types/document";

type DocumentSelectorProps = {
  selectedIds: string[];
  onSelectionChange: (selectedIds: string[]) => void;
  open: boolean;
  onClose: () => void;
};

function DocumentSelector({
  selectedIds,
  onSelectionChange,
  open,
  onClose,
}: DocumentSelectorProps) {
  const {documents, loading} = useDocuments();
  const [localSelectedIds, setLocalSelectedIds] = useState<string[]>(selectedIds);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [loadedDocs, setLoadedDocs] = useState<Map<string, Document>>(new Map());
  const [loadingDocId, setLoadingDocId] = useState<string | null>(null);

  if (!open) return null;

  const handleToggle = (documentId: string) => {
    setLocalSelectedIds((prev) =>
      prev.includes(documentId)
        ? prev.filter((id) => id !== documentId)
        : [...prev, documentId]
    );
  };

  const handleSave = () => {
    onSelectionChange(localSelectedIds);
    onClose();
  };

  const handleCancel = () => {
    setLocalSelectedIds(selectedIds);
    onClose();
  };

  const toggleExpand = (docId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    const newExpandedId = expandedId === docId ? null : docId;
    setExpandedId(newExpandedId);

    // Load document content if expanding and not already loaded
    if (newExpandedId && !loadedDocs.has(newExpandedId)) {
      setLoadingDocId(newExpandedId);
      void api.documents
        .get(newExpandedId)
        .then((doc) => {
          setLoadedDocs((prev) => new Map(prev).set(newExpandedId, doc));
        })
        .catch((err: unknown) => {
          console.error("Failed to load document:", err);
        })
        .finally(() => {
          setLoadingDocId(null);
        });
    }
  };

  const getDocumentContent = (docId: string): string => {
    if (loadingDocId === docId) {
      return "Loading document content...";
    }
    const fullDoc = loadedDocs.get(docId);
    if (fullDoc?.content) {
      return fullDoc.content;
    }
    const listItem = documents.find((d) => d.id === docId);
    return listItem?.summary ?? "No content available";
  };

  const renderDocumentRow = (doc: DocumentListItem) => {
    const isSelected = localSelectedIds.includes(doc.id);
    const isExpanded = expandedId === doc.id;

    return (
      <div key={doc.id} style={{borderBottom: "1px solid #2a2e35"}}>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            padding: "10px 12px",
            cursor: "pointer",
            backgroundColor: isSelected ? "#1e3a5f" : "transparent",
            transition: "background-color 0.15s",
          }}
          onClick={() => handleToggle(doc.id)}
          onMouseEnter={(e) => {
            if (!isSelected) e.currentTarget.style.backgroundColor = "#1f2429";
          }}
          onMouseLeave={(e) => {
            if (!isSelected) e.currentTarget.style.backgroundColor = "transparent";
          }}
        >
          <input
            type="checkbox"
            checked={isSelected}
            onChange={() => handleToggle(doc.id)}
            onClick={(e) => e.stopPropagation()}
            style={{
              marginRight: "12px",
              cursor: "pointer",
              width: "16px",
              height: "16px",
            }}
          />
          <div style={{flex: 1, display: "flex", alignItems: "center", gap: "12px"}}>
            <span
              style={{
                fontWeight: 500,
                color: "#e5e7eb",
                fontSize: "14px",
              }}
            >
              {doc.title}
            </span>
            {doc.doc_type ? (
              <span
                style={{
                  fontSize: "12px",
                  color: "#9ca3af",
                  backgroundColor: "#374151",
                  padding: "2px 8px",
                  borderRadius: "4px",
                }}
              >
                {doc.doc_type}
              </span>
            ) : null}
            {doc.ref_tag ? (
              <span style={{fontSize: "12px", color: "#6b7280"}}>
                {doc.ref_tag}
              </span>
            ) : null}
          </div>
          <button
            type="button"
            onClick={(e) => toggleExpand(doc.id, e)}
            style={{
              background: "none",
              border: "none",
              padding: "8px 12px",
              cursor: "pointer",
              color: "#9ca3af",
              fontSize: "16px",
              display: "flex",
              alignItems: "center",
              transform: isExpanded ? "rotate(90deg)" : "rotate(0deg)",
              transition: "transform 0.2s",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.color = "#e5e7eb";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.color = "#9ca3af";
            }}
          >
            ▶
          </button>
        </div>
        {isExpanded ? (
          <div
            style={{
              padding: "12px 16px 12px 48px",
              backgroundColor: "#1a1d23",
              fontSize: "13px",
              color: "#d1d5db",
              lineHeight: "1.6",
              maxHeight: "300px",
              overflowY: "auto",
              whiteSpace: "pre-wrap",
              borderLeft: "3px solid #374151",
              marginLeft: "12px",
            }}
          >
            <div style={{fontFamily: "monospace", fontSize: "12px"}}>
              {getDocumentContent(doc.id)}
            </div>
          </div>
        ) : null}
      </div>
    );
  };

  return (
    <div
      className="document-selector__overlay"
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor: "rgba(0, 0, 0, 0.75)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 50,
      }}
      onClick={handleCancel}
    >
      <div
        className="document-selector__modal"
        style={{
          backgroundColor: "#1c2028",
          borderRadius: "8px",
          border: "1px solid #2a2e35",
          maxWidth: "700px",
          width: "90%",
          maxHeight: "85vh",
          display: "flex",
          flexDirection: "column",
          boxShadow: "0 25px 50px -12px rgba(0, 0, 0, 0.5)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          style={{
            padding: "20px 24px",
            borderBottom: "1px solid #2a2e35",
          }}
        >
          <h2
            style={{
              fontSize: "18px",
              fontWeight: 600,
              marginBottom: "6px",
              color: "#f3f4f6",
            }}
          >
            Select Documents
          </h2>
          <p style={{fontSize: "13px", color: "#9ca3af"}}>
            Choose documents to attach as agent context ({localSelectedIds.length}{" "}
            selected)
          </p>
        </div>

        <div
          style={{
            flex: 1,
            overflowY: "auto",
            backgroundColor: "#16191f",
            maxHeight: "calc(85vh - 200px)",
          }}
        >
          {loading ? (
            <div
              style={{
                display: "flex",
                justifyContent: "center",
                padding: "60px",
              }}
            >
              <LoadingSpinner size="medium" />
            </div>
          ) : documents.length === 0 ? (
            <div style={{padding: "40px"}}>
              <EmptyState message="No documents available" />
            </div>
          ) : (
            <div>{documents.map(renderDocumentRow)}</div>
          )}
        </div>

        <div
          style={{
            display: "flex",
            justifyContent: "flex-end",
            gap: "12px",
            padding: "16px 24px",
            borderTop: "1px solid #2a2e35",
            backgroundColor: "#1c2028",
          }}
        >
          <button
            type="button"
            onClick={handleCancel}
            style={{
              padding: "8px 16px",
              fontSize: "14px",
              fontWeight: 500,
              color: "#d1d5db",
              backgroundColor: "#374151",
              border: "none",
              borderRadius: "6px",
              cursor: "pointer",
              transition: "background-color 0.15s",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.backgroundColor = "#4b5563";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.backgroundColor = "#374151";
            }}
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleSave}
            style={{
              padding: "8px 16px",
              fontSize: "14px",
              fontWeight: 500,
              color: "#ffffff",
              backgroundColor: "#3b82f6",
              border: "none",
              borderRadius: "6px",
              cursor: "pointer",
              transition: "background-color 0.15s",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.backgroundColor = "#2563eb";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.backgroundColor = "#3b82f6";
            }}
          >
            Save Selection
          </button>
        </div>
      </div>
    </div>
  );
}

export {DocumentSelector};
export type {DocumentSelectorProps};
