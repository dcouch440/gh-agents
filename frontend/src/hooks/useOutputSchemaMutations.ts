import {useState, useCallback} from "react";
import {api} from "@/api";
import {API} from "@/constants";
import {useOutputSchemaContext} from "@/hooks/useOutputSchemaContext";
import type {
  OutputSchema,
  CreateOutputSchemaRequest,
  UpdateOutputSchemaRequest,
} from "@/types/schema";

const useCreateOutputSchema = () => {
  const {addSchema} = useOutputSchemaContext();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const mutate = useCallback(
    async (body: CreateOutputSchemaRequest): Promise<OutputSchema> => {
      setLoading(true);
      setError(null);
      try {
        const schema = await api.post<OutputSchema>(API.OUTPUT_SCHEMAS, body);
        addSchema(schema);
        return schema;
      } catch (e) {
        const msg =
          e instanceof Error ? e.message : "Failed to create output schema";
        setError(msg);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [addSchema],
  );

  return {mutate, loading, error};
};

const useUpdateOutputSchema = () => {
  const {addSchema} = useOutputSchemaContext();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const mutate = useCallback(
    async (
      id: string,
      body: UpdateOutputSchemaRequest,
    ): Promise<OutputSchema> => {
      setLoading(true);
      setError(null);
      try {
        const schema = await api.put<OutputSchema>(API.OUTPUT_SCHEMA(id), body);
        addSchema(schema);
        return schema;
      } catch (e) {
        const msg =
          e instanceof Error ? e.message : "Failed to update output schema";
        setError(msg);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [addSchema],
  );

  return {mutate, loading, error};
};

const useDeleteOutputSchema = () => {
  const {removeSchema} = useOutputSchemaContext();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const mutate = useCallback(
    async (id: string): Promise<void> => {
      setLoading(true);
      setError(null);
      try {
        await api.del(API.OUTPUT_SCHEMA(id));
        removeSchema(id);
      } catch (e) {
        const msg =
          e instanceof Error ? e.message : "Failed to delete output schema";
        setError(msg);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [removeSchema],
  );

  return {mutate, loading, error};
};

export {useCreateOutputSchema, useUpdateOutputSchema, useDeleteOutputSchema};
