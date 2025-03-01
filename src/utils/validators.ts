// src/utils/validators.ts
export const domainValidator = (v: string) => 
  /^(?!-)[A-Za-z0-9-]{1,63}(?<!-)(\.[A-Za-z]{2,})+$/.test(v);

export const portValidator = (v: number) => 
  v > 0 && v < 65536;